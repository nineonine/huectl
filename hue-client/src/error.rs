use reqwest::StatusCode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Couldn't talk to the bridge at all (network, TLS, timeout).
    #[error("request to bridge failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The bridge answered, but not with the JSON envelope we expect.
    #[error("bridge returned HTTP {status} with an unexpected body: {source}")]
    Decode {
        status: StatusCode,
        #[source]
        source: serde_json::Error,
    },

    /// The bridge understood the request and said no, with reasons.
    #[error("bridge returned HTTP {status}: {}", .errors.join("; "))]
    Bridge {
        status: StatusCode,
        errors: Vec<String>,
    },

    /// An error status with a well-formed but empty error list.
    #[error("bridge returned HTTP {0}")]
    Status(StatusCode),
}
