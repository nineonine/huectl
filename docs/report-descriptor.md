# huectl HID reports

The byte-level contract between the controller, the kernel driver and the
daemon. The authoritative copy is the descriptor byte array, currently
`tools/fake-pico/src/descriptor.rs` (DRAFT, moves to the firmware when it
exists; CLAUDE.md decision 4). Keep this file, that array, the driver's
`input_mapping` and the daemon in sync.

Device: VID `0x1209`, PID `0x0001` (pid.codes test PID). Application
collection: Generic Desktop / Multi-axis Controller. All multi-byte fields
are little-endian. Every report starts with its report ID byte.

## Report 1: controls (input, device -> host), 7 bytes

| Byte | Bits | Field | HID usage | Range | Linux event (driver) |
|---|---|---|---|---|---|
| 0 | | Report ID = 1 | | | |
| 1 | 0 | Knob 1 push | Button 1 | 0/1 | `EV_KEY BTN_0` |
| 1 | 1 | Knob 2 push | Button 2 | 0/1 | `EV_KEY BTN_1` |
| 1 | 2 | Knob 3 push | Button 3 | 0/1 | `EV_KEY BTN_2` |
| 1 | 3-5 | Tactile buttons 1-3 | Button 4-6 | 0/1 | `EV_KEY BTN_3..BTN_5` |
| 1 | 6-7 | Padding | (constant) | | |
| 2 | | Knob 1 detents since last report | GD Rx, relative | -127..127 | `EV_REL REL_RX` |
| 3 | | Knob 2 detents since last report | GD Ry, relative | -127..127 | `EV_REL REL_RY` |
| 4 | | Knob 3 detents since last report | GD Rz, relative | -127..127 | `EV_REL REL_RZ` |
| 5-6 | | Slide pot position | GD Slider, absolute | 0..1023 | `EV_ABS ABS_MISC` |

Knob roles are the daemon's choice (CLAUDE.md decision 9): knob 1 =
brightness, knob 2 = hue, knob 3 = saturation. The firmware and driver only
know "knob N".

The HID core also emits `EV_MSC MSC_SCAN` (value = raw usage, e.g.
`0x90001` for Button 1) before each key event. The daemon ignores it.

## Report 2: LEDs (output, host -> device), 2 bytes

| Byte | Field | HID usage | Range |
|---|---|---|---|
| 0 | Report ID = 2 | | |
| 1 | LED bitmask, bit N = LED N on | Vendor 0xFF00:0x01 | 0..255 |

Number and meaning of LEDs: still open (docs/PROGRESS.md).

## Report 3: display state (output, host -> device), 12 bytes

Always the complete state (idempotent, CLAUDE.md decision 3). The daemon
fills it from zigbee2mqtt's state messages and sends it on every change;
the firmware only draws it (decision 10).

| Byte | Field | HID usage | Range |
|---|---|---|---|
| 0 | Report ID = 3 | | |
| 1 | Flags: bit 0 = light on; bit 1 = color mode (1 = hue/saturation, 0 = color temperature); bits 2-7 = 0 | Vendor 0xFF00:0x10 | |
| 2 | Focus, the value being tuned: 0 none, 1 brightness, 2 hue, 3 saturation, 4 color temperature | Vendor 0xFF00:0x11 | 0..4 |
| 3 | Brightness | Vendor 0xFF00:0x12 | 0..254 (zigbee2mqtt scale) |
| 4 | Saturation, percent | Vendor 0xFF00:0x13 | 0..100 |
| 5-6 | Hue, degrees | Vendor 0xFF00:0x14 | 0..359 |
| 7-8 | Color temperature, mireds | Vendor 0xFF00:0x15 | 153..500 |
| 9-11 | Swatch color R, G, B (computed by the daemon) | Vendor 0xFF00:0x16-0x18 | 0..255 each |

Example: light on in color mode, tuning hue, brightness 200, saturation 80,
hue 212, color temperature 366, swatch `#3388ff`:
`03 03 02 c8 50 d4 00 6e 01 33 88 ff`.

## How output reports reach the device

Verified in the VM (2026-10-06): writing a whole report (ID byte first) to
`/dev/hidrawN` delivers it to the device with no driver code. Whether the
daemon uses hidraw for the display, or the driver exposes it another way,
is still open (docs/PROGRESS.md). The LEDs are planned as `led_classdev`.

## Checking a descriptor

- Kernel's parse: `sudo cat /sys/kernel/debug/hid/0003:1209:0001.*/rdesc`
  (in the VM, while fake-pico or the Pico is connected).
- Raw reports: `hid-recorder` from `hid-tools`, or the driver's `dmesg`
  log of each input report.
- Descriptor item data is signed: values >= 128 need the 2-byte form
  (`0x26 0xFF 0x00` = 255, `0x16 0x99 0x00` = 153).
