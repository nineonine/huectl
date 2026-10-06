//! Client for the Philips Hue Bridge CLIP v2 API.
//!
//! Knows Hue; knows nothing about the controller hardware. The daemon
//! connects the two.

mod client;
mod error;
mod types;

pub use client::Bridge;
pub use error::Error;
pub use types::{ColorTemperature, Dimming, Light, LightUpdate, Metadata, On};
