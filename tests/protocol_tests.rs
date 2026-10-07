use axum::{
    body::Bytes,
    extract::State,
    http::{Method, StatusCode},
    response::IntoResponse,
};
use obs_homerun::{
    config::{generate_uuid, Config},
    handlers::{handle_connection_manager, handle_content_directory},
    stream::handle_stream,
};
use std::sync::Arc;

#[test]
fn test_uuid_generation() {
    let uuid1 = generate_uuid("OBS HomeRun");
    let uuid2 = generate_uuid("OBS HomeRun");
    let uuid3 = generate_uuid("Other Name");

    assert_eq!(uuid1, uuid2, "UUIDs for same name must be deterministic");
    assert_ne!(uuid1, uuid3, "UUIDs for different names must differ");

    // RFC 4122 format: 8-4-4-4-12 hex chars
    let parts: Vec<&str> = uuid1.split('-').collect();
    assert_eq!(parts.len(), 5, "UUID must have 5 hyphen-delimited segments");
    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 4);
    assert_eq!(parts[2].len(), 4);
    assert_eq!(parts[3].len(), 4);
    assert_eq!(parts[4].len(), 12);
}

#[test]
fn test_config_defaults() {
    let config = Config::from_env();
    assert_eq!(config.channel_number, "1.1");
    assert_eq!(config.buffer_sec, 3.0);
    assert_eq!(config.half_buffer, "1.5");
    assert_eq!(config.audio_codec, "ac3");
    assert_eq!(config.http_port, 5004);
}

#[test]
fn test_discover_json() {
    let config = Config::from_env();
    let json_str = config.discover_json();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(val["FriendlyName"], config.friendly_name);
    assert_eq!(val["ModelNumber"], "HDTV-1.0");
    assert_eq!(val["TunerCount"], 2);
}

#[test]
fn test_lineup_json() {
    let config = Config::from_env();
    let json_str = config.lineup_json();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON array");

    let arr = val.as_array().expect("Array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["GuideNumber"], config.channel_number);
    assert_eq!(arr[0]["GuideName"], config.friendly_name);
}

#[test]
fn test_device_xml() {
    let config = Config::from_env();
    let xml = config.device_xml();

    assert!(xml.contains("<friendlyName>"));
    assert!(xml.contains(&format!(
        "<friendlyName>{}</friendlyName>",
        config.friendly_name
    )));
    assert!(xml.contains(&format!("<UDN>uuid:{}</UDN>", config.device_uuid)));
    assert!(xml.contains("<modelNumber>HDTV-1.0</modelNumber>"));
}

#[tokio::test]
async fn test_connection_manager_soap() {
    let soap_body = Bytes::from(
        r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetProtocolInfo xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1"/></s:Body></s:Envelope>"#,
    );

    let resp = handle_connection_manager(Method::POST, soap_body).await;
    let body_bytes = axum::body::to_bytes(resp.into_response().into_body(), usize::MAX)
        .await
        .expect("Read response body");
    let body_str = String::from_utf8_lossy(&body_bytes);

    assert!(body_str.contains("GetProtocolInfoResponse"));
    assert!(body_str.contains("video/mpeg"));
}

#[tokio::test]
async fn test_content_directory_soap() {
    let config = Arc::new(Config::from_env());

    // 1. Root browse (ObjectID = 0)
    let soap_root = Bytes::from(
        r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>0</ObjectID><BrowseFlag>BrowseDirectChildren</BrowseFlag></u:Browse></s:Body></s:Envelope>"#,
    );
    let resp_root = handle_content_directory(State(config.clone()), Method::POST, soap_root).await;
    let body_root = axum::body::to_bytes(resp_root.into_response().into_body(), usize::MAX)
        .await
        .expect("Read response body");
    let text_root = String::from_utf8_lossy(&body_root);

    assert!(text_root.contains("<NumberReturned>2</NumberReturned>"));
    assert!(text_root.contains("Tuner_1_0"));

    // 2. Item browse
    let soap_item = Bytes::from(
        r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><ObjectID>v1</ObjectID><BrowseFlag>BrowseMetadata</BrowseFlag></u:Browse></s:Body></s:Envelope>"#,
    );
    let resp_item = handle_content_directory(State(config), Method::POST, soap_item).await;
    let body_item = axum::body::to_bytes(resp_item.into_response().into_body(), usize::MAX)
        .await
        .expect("Read response body");
    let text_item = String::from_utf8_lossy(&body_item);

    assert!(text_item.contains("object.item.videoItem.videoBroadcast"));
    assert!(text_item.contains("DLNA.ORG_OP=00"));
}

#[tokio::test]
async fn test_stream_head() {
    let config = Arc::new(Config::from_env());
    let resp = handle_stream(State(config), Method::HEAD)
        .await
        .expect("HEAD response");

    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("transferMode.dlna.org").unwrap(),
        "Streaming"
    );
    assert_eq!(resp.headers().get("content-type").unwrap(), "video/mpeg");

    let cf = resp
        .headers()
        .get("contentFeatures.dlna.org")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cf.contains("DLNA.ORG_OP=00"));
    assert!(cf.contains("AVC_TS_HD_60_AC3_ISO"));
}
