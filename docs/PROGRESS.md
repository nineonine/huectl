# huectl progress

Update this file at the end of every work block. It is also what gets pasted
into design discussions, so keep it accurate and short.

**Current phase:** 0 (setup)
**Hardware arrived:** no. Being ordered 2026-10-06: CanaKit Pico H Starter Kit (Pico H, breadboard, jumpers, micro-USB cable, LEDs, buttons, resistors), spare Pico H, Debug Probe, KY-040 5-pack, slide pot, ST7789 color display, Zigbee dongle, USB extension cable, multimeter. Already owned (not ordered): USB-C hub, Hue bulb(s). Source of truth: Google Sheet `hue_controller_hardware_list`
**Last updated:** 2026-10-06

## Phase overview

| Phase | Goal | Status |
|---|---|---|
| 0 | Toolchain, kernel dev VM + hello module, Zigbee via MQTT | in progress |
| 1 | Toy daemon: evdev (fake-pico) -> MQTT, coalescing; then real bulbs via zigbee2mqtt | not started |
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
- [ ] **Step 3: Zigbee via MQTT.** (Blocked: dongle not bought.) Install mosquitto + zigbee2mqtt in `dev`, give the container access to the dongle, factory-reset and pair one bulb, toggle and dim it with `mosquitto_pub`.
  *Done when:* a bulb can be switched and dimmed from a shell via MQTT. (Replaces the old "Hue Bridge via curl" step.)
- [x] **Step 4: Kernel dev VM.** Scripted: `distrobox enter dev -- tools/vm.sh create|start|ssh|stop|snapshot|restore|snapshots`. Debian 12 cloud image + cloud-init (no installer), files in `vm/` (gitignored), snapshot `clean`. Kernel 6.1.0-53-amd64, SeaBIOS (no Secure Boot). QEMU/KVM Debian 12 (4 GB RAM, 4 vCPU, 30 GB qcow2), SSH on host port 2222, install `build-essential linux-headers-$(uname -r) git usbutils evtest`. Take a clean snapshot with the VM powered off.
  *Done when:* SSH works and `/lib/modules/$(uname -r)/build` exists.
- [x] **Step 5: Hello-world module.** `driver/hello.c` loads/unloads in the VM; `Dual MIT/GPL` license. `hello.c` with init/exit `pr_info`, `MODULE_LICENSE("GPL")`, Makefile with `obj-m`. `make`, `insmod`, check `dmesg`, `rmmod`.
  *Done when:* load and unload messages appear in `dmesg`. Keep `dmesg -w` open in a second SSH session.
- [~] **Step 6: Firmware toolchain smoke test.** Build half done: `blinky` builds with `cargo +stable build --release --bin blinky` (`+stable` skips Embassy's pinned 1.97 + 14 targets) and converts to a 31 KB `.uf2`. Flashing waits for the board. Clone `embassy-rs/embassy`, build `examples/rp` `blinky` in release mode. Flash and see the LED blink once the board arrives (BOOTSEL + `elf2uf2-rs -d`).
  *Done when:* build succeeds now; LED blinks later.
- [ ] **Step 7: Repo skeleton.** Create the layout from `CLAUDE.md`, `git init`, `.gitignore` for `target/` and `*.ko` build output, commit the hello module under `driver/`. (`git init` + `.gitignore` + hello module done; repo is public (owner OK'd), pushed; origin = `https://github.com/nineonine/huectl` (private); push auth not set up yet.)
  *Done when:* first commit exists.

## Phase 1 preview (no hardware needed)

- [x] ~~`hue-client` crate~~ (Hue Bridge client). Deleted 2026-10-06 after the switch to Zigbee; still in git history at `59a2089`.
- [ ] Toy daemon: fake-pico events (evdev) -> MQTT `zigbee2mqtt/<name>/set`, ~100 ms coalescing, encoder -> `brightness_step`. Testable before the dongle with `mosquitto_sub`.
- [ ] Config file loading (MQTT broker, target light/group name)

## Ahead of Phase 3 (no hardware needed)

- [x] `tools/fake-pico`: virtual controller via `/dev/uhid`, driven by stdin (`press N`, `release N`, `turn D`, `slide V`). Verified in the VM: `hid-generic` binds and `evtest` shows `BTN_0..3`, `REL_DIAL`, `ABS_THROTTLE` (0..1023).
- [x] DRAFT report descriptor in `tools/fake-pico/src/descriptor.rs`: report 1 in = 4 buttons + Dial (rel, i8) + Slider (abs, u16); report 2 out = vendor LED byte. Moves to the firmware later (decision 4).

- [x] Driver step 1: `driver/huectl.c` claims 1209:0001 (beats hid-generic), logs raw reports via `raw_event`, keeps generic input mapping (`HID_CONNECT_DEFAULT`). Test loop: `tools/vm.sh start`, then `distrobox enter dev -- tools/test-driver.sh`. VM snapshot `pre-driver` taken first.
- [x] Driver step 2: own `input_mapping`: encoder push + buttons -> `BTN_0..BTN_3`, encoder -> `REL_DIAL`, slider -> `ABS_MISC` (0..1023 from descriptor), everything else ignored. Verified with evtest.
- [x] Descriptor v2 (spec: `docs/report-descriptor.md`): 6 buttons (3 knob pushes + 3 tactile), 3 relative knobs (Rx/Ry/Rz -> `REL_RX/RY/RZ`), slider; output report 2 (LEDs) and new report 3 (display state, 12 bytes). fake-pico, driver mapping and test script updated; verified in the VM incl. kernel's parse (ct range 153..500) and output reports via `/dev/hidraw`.
- [ ] Driver step 3: `led_classdev` -> output report 2 (fake-pico prints it).

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
| 2026-10-05 | `hue-client::Bridge` takes a caller-built `reqwest::Client` | TLS pinning decided in one place; tests use plain HTTP (superseded) |
| 2026-10-06 | **No Hue Bridge: USB Zigbee dongle on the Deck + zigbee2mqtt; daemon speaks MQTT** (CLAUDE.md decision 8) | Owner's choice. Pico/HID/driver unchanged. Trade-offs accepted: bulbs leave the Hue app; Deck must be on to control them |
| 2026-10-05 | Draft descriptor uses Generic Desktop / Multi-axis Controller app collection | Only some application usages get hid-input mapping; this one gives `BTN_0..` buttons (not mouse/joystick ones) |
| 2026-10-05 | Driver pins event codes: `BTN_0..3`, `REL_DIAL`, `ABS_MISC`; unmapped usages ignored | Contract with the daemon lives in our code, not hid-input guesses; `BTN_*` not `KEY_*` so desktops don't treat it as a keyboard |
| 2026-10-06 | Color via two encoders: hue (wraps 0-360) + saturation; third encoder = brightness (CLAUDE.md decision 9) | Uses parts already in the plan; maps 1:1 to zigbee2mqtt `hue_step` / saturation. Owner picked this over a round trackpad or touchscreen for now |
| 2026-10-06 | ST7789 240x240 color display shows tuned values + color swatch; daemon sends values, firmware draws (decision 10) | Color swatch beats mono SSD1306 for picking colors; keeps firmware free of Hue knowledge |
| 2026-10-06 | Knobs use HID Rx/Ry/Rz -> `REL_RX/RY/RZ` (replaces `REL_DIAL` from 2026-10-05); 6 buttons `BTN_0..5` | Three identical knobs as one code family; nothing consumed `REL_DIAL` yet |
| 2026-10-06 | Display report 3 carries the full state incl. a daemon-computed RGB swatch | Idempotent (decision 3); firmware never converts colors |
| 2026-10-06 | Keep the slide pot; its role is open | Only analog part: ADC + smoothing practice (Phase 4) for $4. Knobs + display already cover the core functions, so it isn't required |
| 2026-10-06 | Buy one spare Pico H besides the starter kit's board | Cheap insurance against wiring mistakes (3.3 V-only GPIOs) |
| 2026-10-05 | `cargo install` tools only inside `dev` | Linked against Debian glibc 2.36, they also run on the newer-glibc host |

## Open questions

- Slide pot: what does it control (color temperature? a fixed "warmth"? nothing yet)? Pickup behavior (ignore until it crosses the bulb value, or jump) only matters if it controls something the bulb can also change elsewhere.
- What does each control do? Decided: encoders = brightness, hue, saturation. Still open: encoder pushes (on/off? reset?), slider (color temperature?), buttons 2-4 (presets?), LEDs (how many, what they show).
- Display report transport: `/dev/hidraw` (verified working in the VM, no driver code) or a driver sysfs attribute (more kernel practice)? Layout is decided (`docs/report-descriptor.md`).
- Control a single bulb, a room/group, or switchable targets?
- Which Zigbee dongle exactly was ordered (ZBDongle-P = zigbee2mqtt adapter `zstack`, ZBDongle-E = `ember`)? Confirm on arrival; it sets the zigbee2mqtt config.
- Which bulbs, how many? Bluetooth-capable (easier factory reset via Hue BT app)?
- One bulb or a zigbee2mqtt group as the default target?
- Pin assignments: decide once parts arrive; record in `docs/pinmap.md`

## Future ideas (not planned yet)

- Finger-drag color control: 40 mm round capacitive trackpad (Cirque GlidePoint Circle, Pinnacle chip, I2C/SPI, absolute X/Y). Angle = hue, radius = saturation. Needs a ribbon-cable (FFC) adapter for the breadboard; connector details unconfirmed.
- Touchscreen (2.4-2.8" SPI TFT + touch controller) showing a color wheel, closest to the Hue app. Most firmware work.
- Thumb joystick: rejected for color (springs back to center, can't hold a value).

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
- reqwest 0.13 renamed the TLS feature: `rustls` (not `rustls-tls`).
- serde: `#[serde(default)]` on a generic `Vec<T>` field demands `T: Default`; use `default = "Vec::new"`.
- Binaries built in `dev` (Debian 12) run in the VM (Debian 12) as is: same glibc.
- Inspect how the kernel parsed a descriptor: `sudo cat /sys/kernel/debug/hid/<bus:vid:pid.N>/rdesc`.
- `insmod` doesn't load dependencies: our module needs `hid.ko`, so `modprobe -a hid uhid` first, or it fails with "Unknown symbol hid_hw_start". (`modprobe` would resolve it, but only finds modules installed under `/lib/modules`.)
- HID core emits `EV_MSC/MSC_SCAN` (value = raw usage, e.g. `0x90001` = Button 1) before each key event. Daemon ignores it.
- HID descriptor item data is signed: 153 as a 1-byte Logical Minimum (`0x15 0x99`) reads as -103. Use the 2-byte form (`0x16 0x99 0x00`). Check with debugfs `rdesc`.
