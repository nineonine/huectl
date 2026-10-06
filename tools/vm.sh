#!/usr/bin/env bash
# Kernel dev VM (Phase 0 / Step 4): Debian 12 under QEMU/KVM.
# Run inside the `dev` distrobox (that's where qemu lives):
#
#   distrobox enter dev -- tools/vm.sh <command>
#
# Commands:
#   create            download Debian 12 cloud image, provision it, snapshot "clean"
#   start             boot in the background (SSH on 127.0.0.1:2222)
#   ssh [cmd...]      SSH into the VM (or run a command there)
#   stop              clean shutdown, waits until QEMU exits
#   snapshot <name>   take an internal qcow2 snapshot (VM must be stopped)
#   restore <name>    roll the disk back to a snapshot (VM must be stopped)
#   snapshots         list snapshots
#
# Everything lives in vm/ (gitignored): disk, SSH key, logs.

set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
vm="$repo/vm"
base="$vm/debian-12-generic-amd64.qcow2"  # pristine download, backing file
disk="$vm/huectl-vm.qcow2"                # overlay we actually boot
key="$vm/id_ed25519"
pidfile="$vm/qemu.pid"
port=2222
img_dir=https://cloud.debian.org/images/cloud/bookworm/latest

die() { echo "vm.sh: $*" >&2; exit 1; }

running() { [ -f "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; }

ssh_vm() {
    ssh -i "$key" -p "$port" \
        -o StrictHostKeyChecking=accept-new \
        -o UserKnownHostsFile="$vm/known_hosts" \
        -o ConnectTimeout=5 \
        debian@127.0.0.1 "$@"
}

# Common QEMU flags. qemu-xhci is a USB 3 controller: the Pico gets passed
# to it later (by host bus/port, see CLAUDE.md).
qemu_args=(
    -enable-kvm -cpu host -m 4G -smp 4
    -drive "file=$disk,if=virtio"
    -nic "user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:$port-:22"
    -device qemu-xhci,id=xhci
    -display none
)

cmd_create() {
    [ -e "$disk" ] && die "$disk already exists; delete vm/ to start over"
    mkdir -p "$vm/seed"

    if [ ! -e "$base" ]; then
        echo "==> Downloading Debian 12 cloud image"
        curl -fL --progress-bar -o "$base.part" "$img_dir/$(basename "$base")"
        curl -fsSL -o "$vm/SHA512SUMS" "$img_dir/SHA512SUMS"
        echo "==> Verifying checksum"
        (cd "$vm" && grep " $(basename "$base")\$" SHA512SUMS \
            | sed "s/$(basename "$base")\$/$(basename "$base").part/" \
            | sha512sum -c -) || die "checksum mismatch"
        mv "$base.part" "$base"
    fi

    # Overlay on top of the pristine image; 30 GB virtual size,
    # cloud-init grows the root partition on first boot.
    qemu-img create -q -f qcow2 -b "$(basename "$base")" -F qcow2 "$disk" 30G

    [ -e "$key" ] || ssh-keygen -q -t ed25519 -N "" -C huectl-vm -f "$key"

    # cloud-init "NoCloud" config, served over HTTP to the guest on first boot.
    cat > "$vm/seed/meta-data" <<EOF
instance-id: huectl-vm-1
local-hostname: huectl-vm
EOF
    # Optional, but cloud-init 22.4 retries a 404 on it, so serve an empty one.
    : > "$vm/seed/vendor-data"
    cat > "$vm/seed/user-data" <<EOF
#cloud-config
users:
  - name: debian
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    ssh_authorized_keys:
      - $(cat "$key.pub")
package_update: true
packages: [build-essential, git, usbutils, evtest, rsync]
runcmd:
  # Headers must match the *running* kernel, hence uname -r at runtime.
  - [sh, -c, 'apt-get install -y linux-headers-\$(uname -r)']
power_state:
  mode: poweroff
  condition: true
EOF

    echo "==> First boot: provisioning (a few minutes; log in vm/firstboot.log)"
    # Request log shows whether the guest actually fetched the seed.
    python3 -m http.server -b 127.0.0.1 -d "$vm/seed" 8000 >"$vm/seed-http.log" 2>&1 &
    http_pid=$!
    trap 'kill $http_pid 2>/dev/null || true' EXIT
    # 10.0.2.2 is the host as seen from QEMU user networking.
    # "nocloud-net" (not "nocloud"): Debian 12's cloud-init 22.4 only fetches
    # a seed over HTTP under that name. The timeout stops a failed boot from
    # idling forever at a login prompt, since only our config powers it off.
    timeout 15m qemu-system-x86_64 "${qemu_args[@]}" \
        -smbios "type=1,serial=ds=nocloud-net;s=http://10.0.2.2:8000/" \
        -serial "file:$vm/firstboot.log" \
        || die "first boot failed or timed out; see vm/firstboot.log and vm/seed-http.log"
    kill $http_pid 2>/dev/null || true

    # Cloud-init says "finished" even when it found no config (DataSourceNone),
    # so check which datasource it used.
    grep -q "finished.*Datasource DataSourceNoCloud" "$vm/firstboot.log" \
        || die "cloud-init didn't use our seed; see vm/firstboot.log and vm/seed-http.log"

    qemu-img snapshot -c clean "$disk"
    echo "==> Done. Snapshot 'clean' taken. Next: tools/vm.sh start"
}

cmd_start() {
    running && die "already running (pid $(cat "$pidfile"))"
    [ -e "$disk" ] || die "no disk; run: tools/vm.sh create"
    qemu-system-x86_64 "${qemu_args[@]}" \
        -serial "file:$vm/serial.log" \
        -daemonize -pidfile "$pidfile"
    echo -n "==> Waiting for SSH"
    for _ in $(seq 60); do
        if ssh_vm true 2>/dev/null; then echo " ok"; return; fi
        echo -n .; sleep 2
    done
    die "SSH did not come up; see vm/serial.log"
}

cmd_stop() {
    running || { echo "not running"; return; }
    ssh_vm sudo poweroff 2>/dev/null || true
    for _ in $(seq 30); do running || { echo "stopped"; return; }; sleep 1; done
    die "VM did not shut down; kill $(cat "$pidfile") if it's really stuck"
}

need_stopped() { running && die "stop the VM first (snapshots need it powered off)"; true; }

case "${1:-}" in
    create)    cmd_create ;;
    start)     cmd_start ;;
    ssh)       shift; ssh_vm "$@" ;;
    stop)      cmd_stop ;;
    snapshot)  need_stopped; qemu-img snapshot -c "${2:?name}" "$disk" ;;
    restore)   need_stopped; qemu-img snapshot -a "${2:?name}" "$disk" ;;
    snapshots) qemu-img snapshot -l -U "$disk" ;;  # -U: works while running
    *)         sed -n '2,17p' "$0"; exit 1 ;;
esac
