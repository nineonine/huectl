//! DRAFT HID report descriptor and report layouts for the controller.
//!
//! Per CLAUDE.md decision 4 the descriptor's home is the firmware; this
//! copy exists so the driver can be developed before the Pico arrives. When
//! the firmware exists, move it there and keep this in sync (or share it).
//! Human-readable spec: docs/report-descriptor.md.
//!
//! Each item is a prefix byte (tag, type, data size) plus 0-2 data bytes.
//! Read it top to bottom: "global" items (usage page, logical min/max,
//! report size/count) set state; "main" items (Input/Output) emit fields
//! using the current state. Item data is SIGNED: values >= 128 need the
//! 2-byte form (e.g. 0x26 0xFF 0x00 for 255; 0x25 0xFF would mean -1).

/// Controls report (device -> host).
pub const REPORT_ID_CONTROLS: u8 = 1;
/// LED report (host -> device).
pub const REPORT_ID_LEDS: u8 = 2;
/// Display report (host -> device).
pub const REPORT_ID_DISPLAY: u8 = 3;

/// Controls report length in bytes, including the report ID.
pub const CONTROLS_LEN: usize = 7;

#[rustfmt::skip]
pub const REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,       // Usage Page (Generic Desktop)
    0x09, 0x08,       // Usage (Multi-axis Controller): makes hid-input map it
    0xA1, 0x01,       // Collection (Application)

    // --- Report 1: controls, device -> host. 7 bytes incl. ID. ---
    0x85, REPORT_ID_CONTROLS, // Report ID (1)

    // Byte 1, bits 0-5: buttons 1-6 (1 = pressed).
    // Buttons 1-3 = knob pushes (knob 1, 2, 3); 4-6 = tactile buttons.
    0x05, 0x09,       //   Usage Page (Button)
    0x19, 0x01,       //   Usage Minimum (Button 1)
    0x29, 0x06,       //   Usage Maximum (Button 6)
    0x15, 0x00,       //   Logical Minimum (0)
    0x25, 0x01,       //   Logical Maximum (1)
    0x75, 0x01,       //   Report Size (1 bit)
    0x95, 0x06,       //   Report Count (6)
    0x81, 0x02,       //   Input (Data, Variable, Absolute)
    // Byte 1, bits 6-7: padding so the next field is byte-aligned.
    0x95, 0x02,       //   Report Count (2)
    0x81, 0x03,       //   Input (Constant)

    // Bytes 2-4: knobs 1-3, detents moved since the last report (relative).
    // Knob 1 = brightness, 2 = hue, 3 = saturation (the daemon decides
    // that; the descriptor only says "three rotations").
    0x05, 0x01,       //   Usage Page (Generic Desktop)
    0x09, 0x33,       //   Usage (Rx) -> knob 1
    0x09, 0x34,       //   Usage (Ry) -> knob 2
    0x09, 0x35,       //   Usage (Rz) -> knob 3
    0x15, 0x81,       //   Logical Minimum (-127)
    0x25, 0x7F,       //   Logical Maximum (127)
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x03,       //   Report Count (3): one field per usage above
    0x81, 0x06,       //   Input (Data, Variable, Relative)

    // Bytes 5-6: slide pot position, little-endian (absolute).
    0x09, 0x36,       //   Usage (Slider)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x03, //   Logical Maximum (1023)
    0x75, 0x10,       //   Report Size (16 bits)
    0x95, 0x01,       //   Report Count (1)
    0x81, 0x02,       //   Input (Data, Variable, Absolute)

    // --- Report 2: LEDs, host -> device. 2 bytes incl. ID. ---
    // Vendor-defined usages: hid-generic ignores them; our driver owns them.
    0x85, REPORT_ID_LEDS, // Report ID (2)
    0x06, 0x00, 0xFF, //   Usage Page (Vendor-defined 0xFF00)
    0x09, 0x01,       //   Usage (0x01: LED bitmask)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x01,       //   Report Count (1)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)

    // --- Report 3: display state, host -> device. 12 bytes incl. ID. ---
    // The full state every time (idempotent, CLAUDE.md decision 3).
    // Usage page is still Vendor 0xFF00 from report 2.
    0x85, REPORT_ID_DISPLAY, // Report ID (3)
    // Bytes 1-3: flags, focus, brightness (one byte each, 0..255 range).
    0x09, 0x10,       //   Usage (0x10: flags; bit 0 on, bit 1 color mode)
    0x09, 0x11,       //   Usage (0x11: focus; 0 none, 1 bri, 2 hue, 3 sat, 4 ct)
    0x09, 0x12,       //   Usage (0x12: brightness, 0..254)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x03,       //   Report Count (3)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)
    // Byte 4: saturation.
    0x09, 0x13,       //   Usage (0x13: saturation)
    0x25, 0x64,       //   Logical Maximum (100)
    0x95, 0x01,       //   Report Count (1)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)
    // Bytes 5-6: hue, little-endian.
    0x09, 0x14,       //   Usage (0x14: hue, degrees)
    0x26, 0x67, 0x01, //   Logical Maximum (359)
    0x75, 0x10,       //   Report Size (16 bits)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)
    // Bytes 7-8: color temperature in mireds, little-endian.
    0x09, 0x15,       //   Usage (0x15: color temperature)
    0x16, 0x99, 0x00, //   Logical Minimum (153); 0x15 0x99 would mean -103
    0x26, 0xF4, 0x01, //   Logical Maximum (500)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)
    // Bytes 9-11: swatch color R, G, B (the daemon converts; firmware draws).
    0x19, 0x16,       //   Usage Minimum (0x16: swatch red)
    0x29, 0x18,       //   Usage Maximum (0x18: swatch blue)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x03,       //   Report Count (3)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)

    0xC0,             // End Collection
];

/// Current state of the physical controls.
#[derive(Debug, Default)]
pub struct Controls {
    /// Bit N = button N+1. Buttons 1-3 are the knob pushes.
    pub buttons: u8,
    /// Detents per knob since the last report; reset to 0 after each send.
    pub knobs: [i8; 3],
    /// 0..=1023.
    pub slider: u16,
}

impl Controls {
    /// The bytes of report 1, exactly as the descriptor lays them out.
    pub fn report(&self) -> [u8; CONTROLS_LEN] {
        let [lo, hi] = self.slider.to_le_bytes();
        let [k1, k2, k3] = self.knobs.map(|k| k as u8);
        [REPORT_ID_CONTROLS, self.buttons & 0x3F, k1, k2, k3, lo, hi]
    }
}

/// Human-readable form of an output report, as the firmware will see it.
pub fn describe_output(data: &[u8]) -> String {
    match data {
        [REPORT_ID_LEDS, mask] => format!("LEDs {mask:08b}"),
        [REPORT_ID_DISPLAY, flags, focus, bri, sat, h0, h1, c0, c1, r, g, b] => {
            let focus = match focus {
                0 => "none",
                1 => "brightness",
                2 => "hue",
                3 => "saturation",
                4 => "color temp",
                _ => "INVALID",
            };
            format!(
                "display: {} {} | focus {focus} | bri {bri} sat {sat} hue {} ct {} | swatch #{r:02x}{g:02x}{b:02x}",
                if flags & 1 != 0 { "on" } else { "off" },
                if flags & 2 != 0 { "color" } else { "white" },
                u16::from_le_bytes([*h0, *h1]),
                u16::from_le_bytes([*c0, *c1]),
            )
        }
        _ => format!("unknown output report {data:02x?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controls_layout_matches_descriptor() {
        let c = Controls { buttons: 0b10_0101, knobs: [-3, 1, 127], slider: 0x0312 };
        // ID, buttons 1+3+6, knobs as two's complement, slider little-endian.
        assert_eq!(c.report(), [1, 0b10_0101, 0xFD, 0x01, 0x7F, 0x12, 0x03]);
    }

    #[test]
    fn display_report_decodes() {
        // on + color mode, focus hue, bri 200, sat 80, hue 212, ct 366, #3388ff
        let r = [3, 0b11, 2, 200, 80, 0xD4, 0x00, 0x6E, 0x01, 0x33, 0x88, 0xFF];
        assert_eq!(
            describe_output(&r),
            "display: on color | focus hue | bri 200 sat 80 hue 212 ct 366 | swatch #3388ff"
        );
    }

    #[test]
    fn report_ids_match_lengths() {
        assert_eq!(describe_output(&[2, 0b101]), "LEDs 00000101");
        assert!(describe_output(&[3, 0]).starts_with("unknown"));
    }
}
