//! DRAFT HID report descriptor and report layout for the controller.
//!
//! Per CLAUDE.md decision 4 the descriptor's home is the firmware; this
//! copy exists so the driver can be developed before the Pico arrives. When
//! the firmware exists, move it there and keep this in sync (or share it).
//!
//! Each item is a prefix byte (tag, type, data size) plus 0-2 data bytes.
//! Read it top to bottom: "global" items (usage page, logical min/max,
//! report size/count) set state; "main" items (Input/Output) emit fields
//! using the current state.

/// Report ID of the controls report (device -> host).
pub const REPORT_ID_CONTROLS: u8 = 1;
/// Report ID of the LED report (host -> device).
pub const REPORT_ID_LEDS: u8 = 2;

#[rustfmt::skip]
pub const REPORT_DESCRIPTOR: &[u8] = &[
    0x05, 0x01,       // Usage Page (Generic Desktop)
    0x09, 0x08,       // Usage (Multi-axis Controller): makes hid-input map it
    0xA1, 0x01,       // Collection (Application)

    // --- Report 1: controls, device -> host. 5 bytes incl. ID. ---
    0x85, REPORT_ID_CONTROLS, // Report ID (1)

    // Byte 1, bits 0-3: buttons 1-4 (1 = pressed). Button 1 = encoder push.
    0x05, 0x09,       //   Usage Page (Button)
    0x19, 0x01,       //   Usage Minimum (Button 1)
    0x29, 0x04,       //   Usage Maximum (Button 4)
    0x15, 0x00,       //   Logical Minimum (0)
    0x25, 0x01,       //   Logical Maximum (1)
    0x75, 0x01,       //   Report Size (1 bit)
    0x95, 0x04,       //   Report Count (4)
    0x81, 0x02,       //   Input (Data, Variable, Absolute)
    // Byte 1, bits 4-7: padding so the next field is byte-aligned.
    0x95, 0x04,       //   Report Count (4)
    0x81, 0x03,       //   Input (Constant)

    // Byte 2: encoder, detents moved since the last report (relative).
    0x05, 0x01,       //   Usage Page (Generic Desktop)
    0x09, 0x37,       //   Usage (Dial) -> REL_DIAL
    0x15, 0x81,       //   Logical Minimum (-127)
    0x25, 0x7F,       //   Logical Maximum (127)
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x01,       //   Report Count (1)
    0x81, 0x06,       //   Input (Data, Variable, Relative)

    // Bytes 3-4: slide pot position, little-endian (absolute).
    0x09, 0x36,       //   Usage (Slider) -> ABS_THROTTLE in hid-input
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x03, //   Logical Maximum (1023), 2-byte data
    0x75, 0x10,       //   Report Size (16 bits)
    0x95, 0x01,       //   Report Count (1)
    0x81, 0x02,       //   Input (Data, Variable, Absolute)

    // --- Report 2: LEDs, host -> device. 2 bytes incl. ID. ---
    // Vendor-defined, so hid-generic ignores it; our driver will own it.
    0x85, REPORT_ID_LEDS, // Report ID (2)
    0x06, 0x00, 0xFF, //   Usage Page (Vendor-defined 0xFF00)
    0x09, 0x01,       //   Usage (0x01: LED bitmask)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255); 0x25 0xFF would mean -1
    0x75, 0x08,       //   Report Size (8 bits)
    0x95, 0x01,       //   Report Count (1)
    0x91, 0x02,       //   Output (Data, Variable, Absolute)

    0xC0,             // End Collection
];

/// Current state of the physical controls.
#[derive(Debug, Default)]
pub struct Controls {
    /// Bit N = button N+1.
    pub buttons: u8,
    /// Detents since the last report; reset to 0 after each send.
    pub dial: i8,
    /// 0..=1023.
    pub slider: u16,
}

impl Controls {
    /// The bytes of report 1, exactly as the descriptor lays them out.
    pub fn report(&self) -> [u8; 5] {
        let [lo, hi] = self.slider.to_le_bytes();
        [REPORT_ID_CONTROLS, self.buttons & 0x0F, self.dial as u8, lo, hi]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_layout_matches_descriptor() {
        let c = Controls { buttons: 0b0101, dial: -3, slider: 0x0312 };
        // ID, buttons 1+3, -3 as two's complement, 0x0312 little-endian.
        assert_eq!(c.report(), [1, 0b0101, 0xFD, 0x12, 0x03]);
    }
}
