//! CLIP v2 resource types.
//!
//! These shapes come from the Hue developer docs and have NOT yet been
//! checked against a real bridge (see docs/PROGRESS.md). Serde ignores
//! fields we don't declare, so a bridge that sends more than this is fine.

use serde::{Deserialize, Serialize};

/// Every CLIP v2 response is wrapped as `{"errors": [...], "data": [...]}`.
#[derive(Debug, Deserialize)]
pub(crate) struct Envelope<T> {
    #[serde(default)]
    pub errors: Vec<ApiError>,
    // Plain `default` would make serde demand `T: Default`; naming
    // Vec::new avoids that bound.
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApiError {
    pub description: String,
}

/// What a write (PUT) returns in `data`: a pointer to the changed resource.
#[derive(Debug, Deserialize)]
pub(crate) struct ResourceRef {
    #[allow(dead_code)] // parsed to validate the shape; not used yet
    pub rid: String,
}

/// One bulb (or plug) as returned by `GET /clip/v2/resource/light`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Light {
    /// Stable UUID; this is what goes in the PUT URL.
    pub id: String,
    pub metadata: Metadata,
    pub on: On,
    /// Absent on lights that can only switch (e.g. smart plugs).
    pub dimming: Option<Dimming>,
    /// Absent on lights without tunable white.
    pub color_temperature: Option<ColorTemperature>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Metadata {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct On {
    pub on: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dimming {
    /// Percent, 0.0 to 100.0.
    pub brightness: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ColorTemperature {
    /// Mired (1,000,000 / kelvin); higher is warmer. `None` while the
    /// light is in color (xy) mode.
    pub mirek: Option<u16>,
}

/// Body for `PUT /clip/v2/resource/light/{id}`.
///
/// Only fields that are set get serialized, so an update never touches
/// state we didn't mean to change. Build with the chained helpers:
/// `LightUpdate::default().on(true).brightness(40.0)`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LightUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<On>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimming: Option<Dimming>,
}

impl LightUpdate {
    pub fn on(self, on: bool) -> Self {
        Self { on: Some(On { on }), ..self }
    }

    /// Clamped to 0..=100, since the bridge rejects anything outside it.
    pub fn brightness(self, percent: f64) -> Self {
        let brightness = percent.clamp(0.0, 100.0);
        Self { dimming: Some(Dimming { brightness }), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn update_serializes_only_set_fields() {
        let body = serde_json::to_value(LightUpdate::default().brightness(42.0)).unwrap();
        assert_eq!(body, json!({ "dimming": { "brightness": 42.0 } }));
    }

    #[test]
    fn brightness_is_clamped() {
        assert_eq!(LightUpdate::default().brightness(150.0).dimming.unwrap().brightness, 100.0);
        assert_eq!(LightUpdate::default().brightness(-5.0).dimming.unwrap().brightness, 0.0);
    }
}
