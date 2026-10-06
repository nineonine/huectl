# huectl progress

Update this file at the end of every work block. It is also what gets pasted
into design discussions, so keep it accurate and short.

**Current phase:** 0 (setup)
**Hardware arrived:** no
**Last updated:** 2026-10-05

## Phase overview

| Phase | Goal | Status |
|---|---|---|
| 0 | Toolchain, kernel dev VM + hello module, Hue API via curl | in progress |
| 1 | `hue-client` crate + toy daemon driven by keyboard events | not started |
| 2 | Firmware: Pico enumerates, one button reaches `evtest` (stock HID) | not started |
| 3 | Custom report descriptor + C `hid_driver`; test on `uhid` fake, then real board | not started |
| 4 | Encoder, slide pot, debouncing, smoothing | not started |
| 5 | Output path: bulb state back to LEDs via output reports | not started |
| 6 | DKMS, udev, systemd, enclosure | not started |

## Phase 0 checklist

- [x] **Step 1: Prep the Deck.** Scripted as `tools/prep-deck.sh` (read-only checks: `/dev/kvm`, 30+ GB free, RAM, distrobox/podman, sudo password, git identity). Repo lives at `~/Projects/huectl`.
  *Done when:* `tools/prep-deck.sh` exits 0.
- [x] **Step 2: Dev container.** (Rust 1.99, probe-rs 0.32.0, elf2uf2-rs, flip-link 0.1.12; tools also run on host) `distrobox create -n dev -i debian:12`; inside it install `build-essential git curl pkg-config libssl-dev libudev-dev`, rustup, `rustup target add thumbv6m-none-eabi`, `cargo install probe-rs-tools elf2uf2-rs flip-link`.
  *Done when:* `cargo --version` and `probe-rs --version` work inside `dev`.
- [ ] **Step 3: Hue Bridge via curl.** (Blocked: no bridge yet.) Find bridge IP, press link button and request an application key, list lights, toggle one, change brightness.
  *Done when:* a bulb can be switched and dimmed from a shell. Key stored outside the repo.
- [x] **Step 4: Kernel dev VM.** Scripted: `distrobox enter dev -- tools/vm.sh create|start|ssh|stop|snapshot|restore|snapshots`. Debian 12 cloud image + cloud-init (no installer), files in `vm/` (gitignored), snapshot `clean`. Kernel 6.1.0-53-amd64, SeaBIOS (no Secure Boot). QEMU/KVM Debian 12 (4 GB RAM, 4 vCPU, 30 GB qcow2), SSH on host port 2222, install `build-essential linux-headers-$(uname -r) git usbutils evtest`. Take a clean snapshot with the VM powered off.
  *Done when:* SSH works and `/lib/modules/$(uname -r)/build` exists.
- [x] **Step 5: Hello-world module.** `driver/hello.c` loads/unloads in the VM; `Dual MIT/GPL` license. `hello.c` with init/exit `pr_info`, `MODULE_LICENSE("GPL")`, Makefile with `obj-m`. `make`, `insmod`, check `dmesg`, `rmmod`.
  *Done when:* load and unload messages appear in `dmesg`. Keep `dmesg -w` open in a second SSH session.
- [~] **Step 6: Firmware toolchain smoke test.** Build half done: `blinky` builds with `cargo +stable build --release --bin blinky` (`+stable` skips Embassy's pinned 1.97 + 14 targets) and converts to a 31 KB `.uf2`. Flashing waits for the board. Clone `embassy-rs/embassy`, build `examples/rp` `blinky` in release mode. Flash and see the LED blink once the board arrives (BOOTSEL + `elf2uf2-rs -d`).
  *Done when:* build succeeds now; LED blinks later.
- [ ] **Step 7: Repo skeleton.** Create the layout from `CLAUDE.md`, `git init`, `.gitignore` for `target/` and `*.ko` build output, commit the hello module under `driver/`. (`git init` + `.gitignore` + hello module done; repo is public (owner OK'd), pushed; origin = `https://github.com/nineonine/huectl` (private); push auth not set up yet.)
  *Done when:* first commit exists.

## Phase 1 preview (no hardware needed)

- [ ] `hue-client` crate: discover bridge, list lights, set on/off and brightness, with tests, verified against the real bridge
- [ ] Pinned-certificate handling for the bridge's self-signed cert
- [ ] Toy daemon: keyboard events (evdev) -> Hue commands, with ~100 ms coalescing
- [ ] Config file loading (bridge IP, application key path)

## Decisions log

| Date | Decision | Why |
|---|---|---|
| | RP2040 Pico H (not Pico 2) | Embassy and tutorials most mature on RP2040 |
| | Kernel driver in C, rest in Rust | No usable Rust abstractions for HID/input |
| | Kernel work only in a VM | Safety; avoids SteamOS custom kernel header issues |
| | Hybrid report descriptor | Standard usages for inputs, vendor-defined for outputs |
| | Idempotent output reports | Robust against dropped or reordered messages |
| | Daemon holds all policy | Keeps firmware and kernel simple |
| 2026-10-05 | `dev` container shares `$HOME`, so it shares `~/.rustup`/`~/.cargo` with the host | One toolchain; no duplication |
| 2026-10-05 | VM from Debian cloud image + cloud-init NoCloud seed over HTTP | Scriptable, reproducible, no installer clicking |
| 2026-10-05 | VM files live in repo `vm/` (gitignored), dedicated SSH key there | Keeps everything inside the repo, nothing in `~/.ssh` |
| 2026-10-05 | Kernel code licensed `Dual MIT/GPL` | Repo is MIT; kernel needs GPL-compatible to use GPL-only symbols |
| 2026-10-05 | `cargo install` tools only inside `dev` | Linked against Debian glibc 2.36, they also run on the newer-glibc host |

## Open questions

- Slide pot "pickup" behavior: ignore until it crosses the bulb value, or jump?
- What does each control do? (Encoder 1 = brightness; push = on/off; second encoder? slider = color temperature; buttons = scenes?)
- Control a single bulb, a room/group, or switchable targets?
- Bridge-less Zigbee route: out of scope for now (revisit only if wanted)
- Pin assignments: decide once parts arrive; record in `docs/pinmap.md`

## Pin map

TBD. Create `docs/pinmap.md` when the hardware arrives.

## Notes and gotchas discovered

(Add things learned along the way: errors hit, fixes, surprises.)

- `lscpu` on the Deck prints no "Virtualization" line, but `/dev/kvm` works. Trust `/dev/kvm`.
- Host rustup was 1.67 (2023), far too old for Embassy; updated to stable 1.99.
- `.gitignore` ignores `config.toml` (secrets) but re-includes `.cargo/config.toml` (Embassy needs it).
- Debian 12's cloud-init 22.4 needs `ds=nocloud-net` (not `nocloud`) for an HTTP seed; otherwise it silently falls back to `DataSourceNone` and still prints "finished". Check the datasource name.
- cloud-init 22.4 retries a missing `vendor-data` every second; serve an empty one.
- `distrobox enter dev -- cmd` re-`eval`s its args, so pipes/quotes break. Feed multi-line remote commands as `tools/vm.sh ssh 'bash -s' <<'EOF'`.
- `gh` exists only in `dev`, so `git push` works only from inside `dev`.
- In the VM, `modinfo`/`insmod` live in `/sbin` (not on a normal user's PATH).
- Expected on insmod: "out-of-tree module taints kernel" and "signature ... missing". Fine without Secure Boot.
