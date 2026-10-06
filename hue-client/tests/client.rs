//! Exercises the real HTTP path against a local fake bridge (wiremock).
//!
//! tests/fixtures/lights.json is hand-written from the Hue docs. Replace it
//! with a captured response once a real bridge is available.

use hue_client::{Bridge, Error, LightUpdate};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "test-app-key";
const LAMP: &str = "3f1c9a2e-0d6b-4c55-9e1a-7b2f4d8c6a01";

async fn bridge_for(server: &MockServer) -> Bridge {
    Bridge::new(reqwest::Client::new(), server.uri(), KEY)
}

#[tokio::test]
async fn lights_parses_bridge_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/clip/v2/resource/light"))
        .and(header("hue-application-key", KEY))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/lights.json"), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let lights = bridge_for(&server).await.lights().await.unwrap();

    assert_eq!(lights.len(), 2);
    let lamp = &lights[0];
    assert_eq!(lamp.id, LAMP);
    assert_eq!(lamp.metadata.name, "Desk lamp");
    assert!(lamp.on.on);
    assert_eq!(lamp.dimming.unwrap().brightness, 62.5);
    assert_eq!(lamp.color_temperature.unwrap().mirek, Some(366));

    // A plug: no dimming, no color temperature.
    let plug = &lights[1];
    assert!(!plug.on.on);
    assert!(plug.dimming.is_none());
    assert!(plug.color_temperature.is_none());
}

#[tokio::test]
async fn set_light_sends_only_changed_fields() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path(format!("/clip/v2/resource/light/{LAMP}")))
        .and(header("hue-application-key", KEY))
        // Exact body match: no "on" field, since we didn't set it.
        .and(body_json(json!({ "dimming": { "brightness": 40.0 } })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [],
            "data": [{ "rid": LAMP, "rtype": "light" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    bridge_for(&server)
        .await
        .set_light(LAMP, &LightUpdate::default().brightness(40.0))
        .await
        .unwrap();
}

#[tokio::test]
async fn bridge_error_descriptions_are_surfaced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(403).set_body_json(json!({
            "errors": [{ "description": "unauthorized user" }],
            "data": []
        })))
        .mount(&server)
        .await;

    let err = bridge_for(&server).await.lights().await.unwrap_err();

    match &err {
        Error::Bridge { status, errors } => {
            assert_eq!(status.as_u16(), 403);
            assert_eq!(errors, &["unauthorized user"]);
        }
        other => panic!("expected Error::Bridge, got {other:?}"),
    }
    assert_eq!(err.to_string(), "bridge returned HTTP 403 Forbidden: unauthorized user");
}

#[tokio::test]
async fn non_json_body_is_a_decode_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string("<html>Not Found</html>"))
        .mount(&server)
        .await;

    let err = bridge_for(&server).await.lights().await.unwrap_err();

    assert!(matches!(err, Error::Decode { status, .. } if status.as_u16() == 404), "{err:?}");
}
