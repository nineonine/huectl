use std::fmt;

use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;

use crate::Error;
use crate::types::{Envelope, Light, LightUpdate, ResourceRef};

/// Header carrying the application key on every CLIP v2 request.
const KEY_HEADER: &str = "hue-application-key";

/// A paired Hue Bridge.
pub struct Bridge {
    http: Client,
    base: String,
    key: String,
}

impl Bridge {
    /// `base` is the scheme and host, e.g. `https://192.168.1.10`.
    ///
    /// The HTTP client is passed in rather than built here, so the TLS
    /// policy (pinning the bridge's self-signed cert) is decided in one
    /// place by the caller, and tests can point at a plain-HTTP fake.
    pub fn new(http: Client, base: impl Into<String>, key: impl Into<String>) -> Self {
        let base = base.into().trim_end_matches('/').to_string();
        Self { http, base, key: key.into() }
    }

    /// All lights known to the bridge.
    pub async fn lights(&self) -> Result<Vec<Light>, Error> {
        let url = format!("{}/clip/v2/resource/light", self.base);
        self.send(self.http.get(url)).await
    }

    /// Change one light. Only the fields set in `update` are sent.
    pub async fn set_light(&self, id: &str, update: &LightUpdate) -> Result<(), Error> {
        let url = format!("{}/clip/v2/resource/light/{id}", self.base);
        let _: Vec<ResourceRef> = self.send(self.http.put(url).json(update)).await?;
        Ok(())
    }

    /// Send a request and unwrap the `{errors, data}` envelope.
    async fn send<T: DeserializeOwned>(&self, req: RequestBuilder) -> Result<Vec<T>, Error> {
        let resp = req.header(KEY_HEADER, &self.key).send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        let envelope: Envelope<T> =
            serde_json::from_str(&body).map_err(|source| Error::Decode { status, source })?;
        if !envelope.errors.is_empty() {
            let errors = envelope.errors.into_iter().map(|e| e.description).collect();
            return Err(Error::Bridge { status, errors });
        }
        if !status.is_success() {
            return Err(Error::Status(status));
        }
        Ok(envelope.data)
    }
}

/// Hand-written so the application key never ends up in logs via `{:?}`.
impl fmt::Debug for Bridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bridge")
            .field("base", &self.base)
            .field("key", &"<redacted>")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_does_not_leak_key() {
        let bridge = Bridge::new(Client::new(), "https://bridge", "s3cret-key");
        let shown = format!("{bridge:?}");
        assert!(!shown.contains("s3cret-key"), "{shown}");
        assert!(shown.contains("<redacted>"));
    }
}
