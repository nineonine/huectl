# huectl

A hardware controller for Philips Hue bulbs on Linux. A Raspberry Pi Pico
(RP2040) with an encoder, slide pot, buttons and LEDs talks USB HID to a
custom C kernel driver, which exposes standard input and LED interfaces. A
Rust daemon turns those into Zigbee commands via zigbee2mqtt and a USB
Zigbee coordinator dongle. No Hue Bridge.

The owner is new to hardware and wants to learn by building each layer.
Explain decisions briefly as you go, keep changes small and reviewable, and
prefer showing how to verify something over asserting it works.

Current status and next steps live in `docs/PROGRESS.md`. Read it first and
update it at the end of every work block.

## Architecture

```
Pico (Rust, Embassy firmware)
  -> USB HID (input reports up, output reports down)
  -> custom C hid_driver kernel module
       |- input_dev     -> /dev/input/eventN
       '- led_classdev  -> /sys/class/leds/huectl::*
  -> Rust tokio daemon (evdev in, sysfs out)
  -> MQTT (mosquitto) -> zigbee2mqtt -> USB Zigbee coordinator dongle
  -> Zigbee -> Hue bulbs
```

Division of responsibility:
- **Firmware:** dumb. Debounce, filter, build reports.
- **Kernel driver:** thin. Parse reports, expose standard interfaces.
- **Daemon:** smart. Mapping, rate limiting, policy, light/Zigbee knowledge.
- No light/Zigbee knowledge ever goes in the firmware or kernel.

## Environment (read carefully)

- Development machine is a **Steam Deck** (SteamOS, read-only root).
  - Do NOT modify the host outside the home directory.
  - Do NOT run `steamos-readonly disable` or install system packages on the host.
- Userspace tooling (Rust, probe-rs, cross-compile targets, daemon) lives in a
  **Distrobox container**, Debian 12, named `dev`.
- **Kernel work happens ONLY inside a QEMU/KVM Debian 12 VM** (SSH via host
  port 2222 -> guest 22). Never build or load modules on the host.
- Take a qcow2 snapshot of the VM before loading experimental modules.
- Flashing and `probe-rs` run on the host or the dev container. Only the HID
  device is passed through to the VM.
- Pass the Pico to the VM by **host bus/port**, not VID/PID, because BOOTSEL
  mode changes the USB ID.
- zigbee2mqtt and mosquitto run in the `dev` container. The Zigbee dongle
  stays on the host (not passed to the VM); the container sees it as a
  serial device.
- Hardware may not have arrived yet. Check `docs/PROGRESS.md`.

### Hardware
Pico H x2 (RP2040), Raspberry Pi Debug Probe (SWD), KY-040 rotary encoders,
10k linear slide pot, tactile buttons, LEDs + 330 ohm resistors, breadboard.
Optional later: SSD1306 I2C OLED.
Zigbee coordinator USB dongle (model not yet chosen; see `docs/PROGRESS.md`).

## Repo layout

```
huectl/
  CLAUDE.md
  firmware/    Rust, Embassy, target thumbv6m-none-eabi (OUTSIDE the main workspace)
  driver/      C kernel module, Makefile, dkms.conf
  daemon/      Rust, tokio
  hue-client/  Rust lib: Hue Bridge CLIP v2 client. SUPERSEDED by the
               Zigbee route; kept for reference until the MQTT client exists
  docs/        PROGRESS.md, report descriptor spec, pin map
  tools/       udev rules, systemd unit, test scripts
```

`firmware/` builds for a different target than the host crates, so keep it
out of the main Cargo workspace (or give it its own target config).

## Tech stack

| Layer | Language | Key pieces |
|---|---|---|
| Firmware | Rust | `embassy-rp`, `embassy-usb`, `embassy-executor`, `embassy-sync`, `defmt`, `defmt-rtt`, `flip-link`; `probe-rs` and `elf2uf2-rs` for flashing |
| Kernel driver | C | `struct hid_driver`, input subsystem, LED class |
| Daemon | Rust | `tokio`, `evdev`, `rumqttc` (MQTT), `serde`, `serde_json`, `toml`, `tracing`, `sd-notify` |
| Light control | (external) | `zigbee2mqtt` + `mosquitto` in the `dev` container, USB Zigbee coordinator |
| Fake device | Rust | `/dev/uhid` emulator for testing the driver without hardware |

## Design decisions already made (do not re-litigate without asking)

1. **Relative controls** (encoder) for brightness. The slide pot is absolute,
   so it needs a "pickup" behavior (ignore until it crosses the bulb's current
   value). Pickup behavior is still undecided; see open questions.
2. **Hybrid HID report descriptor:** standard usages for inputs, vendor-defined
   usages for output reports (LEDs/display).
3. **Output reports are idempotent** ("set LED to X"), never "toggle".
4. The **HID report descriptor is the single source of truth.** It lives as a
   documented byte array in the firmware; mirror field offsets in a C header;
   verify with `hid-recorder`.
5. Kernel driver is **C** (the HID and input subsystems have essentially no
   Rust abstractions). Firmware and daemon are Rust.
6. Develop with VID `0x1209` / PID `0x0001` (pid.codes test PID).
7. The driver's `id_table` must match the VID/PID, otherwise `hid-generic`
   binds first.
8. **No Hue Bridge.** Bulbs are controlled over Zigbee through a USB
   coordinator dongle on the Deck, via zigbee2mqtt; the daemon speaks MQTT.
   Accepted trade-offs: the bulbs leave the Hue app (no app, voice or
   schedules), and the Deck must be on to control them.

## Constraints and gotchas

- **Zigbee throughput is limited.** Coalesce encoder input into one command
  per ~100 ms. For several bulbs prefer a Zigbee group (one multicast) over
  per-bulb commands.
- **2.4 GHz interference:** USB 3 ports and Wi-Fi disturb Zigbee. Put the
  dongle on a short USB extension cable, away from the Deck and USB 3 hubs.
- **The Deck has one USB-C port.** Pico, Debug Probe and dongle together
  need a (powered) USB hub or dock.
- **Serial access:** the dongle appears as `/dev/ttyUSB*` or `/dev/ttyACM*`
  on the host; access needs a udev rule or group membership (needs `sudo`:
  ask first).
- **Pairing Hue bulbs:** bulbs previously paired to a bridge must be factory
  reset first (zigbee2mqtt Touchlink reset with the dongle ~10 cm from the
  bulb, or the Hue Bluetooth app).
- **Secrets:** the zigbee2mqtt network key lives in its own data dir outside
  the repo. Never commit it.
- **Pico GPIOs are 3.3 V only**, not 5 V tolerant. Power modules from 3V3.
- **Encoders bounce.** Debounce in firmware. Slide pot ADC is noisy: smooth it
  (moving average or hysteresis).
- **Kernel context rules:** know when you may sleep (`GFP_KERNEL` vs
  `GFP_ATOMIC`), use `devm_*` allocation, return negative errno, log with
  `dev_info`/`dev_err`.
- **Secure Boot** (inside the VM) may require module signing; disable it for
  the dev VM if needed.

## zigbee2mqtt essentials (VERIFY against the docs and real messages)

These were written from memory. Confirm every topic and payload against the
zigbee2mqtt docs and `mosquitto_sub -v -t 'zigbee2mqtt/#'` before relying
on them.

- Set state: publish to `zigbee2mqtt/<friendly_name>/set`, e.g.
  `{"state": "ON", "brightness": 0..254, "color_temp": <mireds>, "transition": 0.2}`.
- Relative dimming: `{"brightness_step": N}` (signed), a natural fit for
  encoder detents.
- State/feedback: zigbee2mqtt publishes each device's state to
  `zigbee2mqtt/<friendly_name>` (use it for LEDs and slider pickup).
- Device list: retained message on `zigbee2mqtt/bridge/devices`.
- Pairing: `zigbee2mqtt/bridge/request/permit_join`; Touchlink factory reset
  via `zigbee2mqtt/bridge/request/touchlink/factory_reset`.
- Groups: defined in zigbee2mqtt, addressed like devices by friendly name.

## Working agreements

- **Ask before** installing anything, using `sudo`, or doing anything outside
  the repo, the `dev` container, or the VM.
- Commit small, one concern per commit, with clear messages.
- After each step, say how to verify it ("done when ...").
- Don't guess hardware details (pins, dongle model/port, bulb models, keys). Ask.
- Prefer tests where practical, especially in the daemon's MQTT layer.
- If something fails, show the actual error and explain it; don't paper over it.
- When a design decision is made, record it in `docs/PROGRESS.md`.

## Reference material

- Kernel: `drivers/hid/hid-led.c` (small, has an output path), `hid-picolcd.c`,
  `Documentation/hid/`, `Documentation/input/`
- Embassy: `embassy-rs/embassy`, `examples/rp`
- Tools: `hid-tools` (`hid-recorder`, `hid-replay`), `evtest`, `usbmon` + Wireshark
- Reading: "USB in a NutShell" (Beyond Logic), HID spec and Usage Tables
  (usb.org), Bootlin kernel/driver slides, zigbee2mqtt docs (zigbee2mqtt.io)