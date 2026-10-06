#!/usr/bin/env bash
# Build driver/ in the VM, load it, drive it with fake-pico, show results.
# The VM must be running (tools/vm.sh start). Run inside the dev container:
#
#   distrobox enter dev -- tools/test-driver.sh
#
# Done when: "bound driver: huectl", the raw reports appear in dmesg, and
# evtest shows the expected events.

set -euo pipefail

repo="$(cd "$(dirname "$0")/.." && pwd)"
vm="$repo/tools/vm.sh"

echo "==> Building fake-pico"
(cd "$repo" && cargo build -q --release -p fake-pico)

echo "==> Copying driver/ and fake-pico to the VM"
"$vm" ssh 'rm -rf huectl && mkdir -p huectl'
tar -C "$repo" -cf - driver | "$vm" ssh 'tar -C huectl -xf -'
"$vm" ssh 'cat > huectl/fake-pico && chmod +x huectl/fake-pico' \
    < "$repo/target/release/fake-pico"

"$vm" ssh 'bash -s' <<'EOF'
set -euo pipefail
cd ~/huectl

echo "==> Building module"
make -C driver -s 2>&1 | grep -v 'Skipping BTF' || true

sudo rmmod huectl 2>/dev/null || true
sudo dmesg -C
# insmod doesn't resolve dependencies; the HID core (hid.ko) and uhid
# must already be loaded or insmod fails with "Unknown symbol".
sudo modprobe -a hid uhid
sudo insmod driver/huectl.ko
echo "==> Loaded: $(lsmod | grep '^huectl' || echo 'NOT LOADED')"

rm -f /tmp/ctl; mkfifo /tmp/ctl
sudo ./fake-pico < /tmp/ctl 2> /tmp/fake.log &
exec 3>/tmp/ctl     # hold the pipe open so fake-pico doesn't see EOF
sleep 1

hid=$(ls -d /sys/bus/hid/devices/0003:1209:0001.* | head -1)
echo "==> HID device: $(basename "$hid")"
echo "==> bound driver: $(basename "$(readlink -f "$hid/driver")" 2>/dev/null || echo none)"

ev=$(ls "$hid"/input/input*/ | grep '^event' | head -1 || true)
if [ -n "$ev" ]; then
    sudo timeout 3 evtest "/dev/input/$ev" > /tmp/evtest.log 2>&1 &
    sleep 1
fi
for c in "press 1" "release 1" "turn 3" "turn -2" "slide 512"; do
    echo "$c" >&3; sleep 0.2
done
sleep 2
echo quit >&3; exec 3>&-; wait || true

sudo rmmod huectl
echo "==> dmesg"; sudo dmesg
echo "==> evtest (/dev/input/${ev:-none})"
grep -E '^Event:' /tmp/evtest.log | grep -v SYN_REPORT || echo "(no events)"
EOF
