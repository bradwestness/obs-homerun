use crate::config::Config;
use axum::{
    body::Bytes,
    extract::State,
    http::{header, Method, Response, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

pub const CONNECTION_MANAGER_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>GetProtocolInfo</name>
      <argumentList>
        <argument><name>Source</name><direction>out</direction><relatedStateVariable>SourceProtocolInfo</relatedStateVariable></argument>
        <argument><name>Sink</name><direction>out</direction><relatedStateVariable>SinkProtocolInfo</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetCurrentConnectionIDs</name>
      <argumentList>
        <argument><name>ConnectionIDs</name><direction>out</direction><relatedStateVariable>CurrentConnectionIDs</relatedStateVariable></argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="yes"><name>SourceProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>SinkProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>CurrentConnectionIDs</name><dataType>string</dataType></stateVariable>
  </serviceStateTable>
</scpd>"#;

pub const CONTENT_DIRECTORY_XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>Browse</name>
      <argumentList>
        <argument><name>ObjectID</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_ObjectID</relatedStateVariable></argument>
        <argument><name>BrowseFlag</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_BrowseFlag</relatedStateVariable></argument>
        <argument><name>Filter</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Filter</relatedStateVariable></argument>
        <argument><name>StartingIndex</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Index</relatedStateVariable></argument>
        <argument><name>RequestedCount</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>SortCriteria</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_SortCriteria</relatedStateVariable></argument>
        <argument><name>Result</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Result</relatedStateVariable></argument>
        <argument><name>NumberReturned</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>TotalMatches</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>UpdateID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_UpdateID</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetSearchCapabilities</name>
      <argumentList><argument><name>SearchCaps</name><direction>out</direction><relatedStateVariable>SearchCapabilities</relatedStateVariable></argument></argumentList>
    </action>
    <action>
      <name>GetSortCapabilities</name>
      <argumentList><argument><name>SortCaps</name><direction>out</direction><relatedStateVariable>SortCapabilities</relatedStateVariable></argument></argumentList>
    </action>
    <action>
      <name>GetSystemUpdateID</name>
      <argumentList><argument><name>Id</name><direction>out</direction><relatedStateVariable>SystemUpdateID</relatedStateVariable></argument></argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_SortCriteria</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_UpdateID</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Filter</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Result</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Index</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ObjectID</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SortCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SearchCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Count</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_BrowseFlag</name>
      <dataType>string</dataType>
      <allowedValueList><allowedValue>BrowseMetadata</allowedValue><allowedValue>BrowseDirectChildren</allowedValue></allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="yes"><name>SystemUpdateID</name><dataType>ui4</dataType></stateVariable>
  </serviceStateTable>
</scpd>"#;

pub async fn handle_discover(State(config): State<Arc<Config>>) -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(config.discover_json())
        .unwrap()
}

pub async fn handle_lineup(State(config): State<Arc<Config>>) -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(config.lineup_json())
        .unwrap()
}

pub async fn handle_device_xml(State(config): State<Arc<Config>>) -> impl IntoResponse {
    Response::builder()
        .header(header::CONTENT_TYPE, "text/xml; charset=utf-8")
        .body(config.device_xml())
        .unwrap()
}

pub async fn handle_connection_manager(method: Method, body: Bytes) -> impl IntoResponse {
    if method == Method::POST {
        let body_str = String::from_utf8_lossy(&body);
        let resp = if body_str.contains("GetProtocolInfo") {
            r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetProtocolInfoResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<Source>http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000,http-get:*:video/mpeg:*,http-get:*:video/mp2t:*,http-get:*:video/vnd.dlna.mpeg-tts:*</Source>
<Sink></Sink>
</u:GetProtocolInfoResponse>
</s:Body>
</s:Envelope>"#
        } else {
            r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetCurrentConnectionIDsResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<ConnectionIDs>0</ConnectionIDs>
</u:GetCurrentConnectionIDsResponse>
</s:Body>
</s:Envelope>"#
        };

        return Response::builder()
            .header(header::CONTENT_TYPE, "text/xml; charset=utf-8")
            .body(resp.to_string())
            .unwrap();
    }

    Response::builder()
        .header(header::CONTENT_TYPE, "text/xml; charset=utf-8")
        .body(CONNECTION_MANAGER_XML.to_string())
        .unwrap()
}

pub async fn handle_content_directory(
    State(config): State<Arc<Config>>,
    method: Method,
    body: Bytes,
) -> impl IntoResponse {
    if method == Method::POST {
        let body_str = String::from_utf8_lossy(&body);
        let (resp, status) = if body_str.contains("GetSystemUpdateID") {
            (
                r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSystemUpdateIDResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><Id>1</Id></u:GetSystemUpdateIDResponse></s:Body>
</s:Envelope>"#.to_string(),
                StatusCode::OK,
            )
        } else if body_str.contains("GetSortCapabilities") {
            (
                r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSortCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><SortCaps></SortCaps></u:GetSortCapabilitiesResponse></s:Body>
</s:Envelope>"#.to_string(),
                StatusCode::OK,
            )
        } else if body_str.contains("GetSearchCapabilities") {
            (
                r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body><u:GetSearchCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"><SearchCaps></SearchCaps></u:GetSearchCapabilitiesResponse></s:Body>
</s:Envelope>"#.to_string(),
                StatusCode::OK,
            )
        } else if body_str.contains("Browse") {
            let (result, count) =
                if body_str.contains("ObjectID>0<") || body_str.contains("ObjectID&gt;0&lt;") {
                    (config.didl_containers(), 2)
                } else {
                    (config.didl_item(), 1)
                };

            (
                format!(
                    r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:BrowseResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<Result>{result}</Result>
<NumberReturned>{count}</NumberReturned>
<TotalMatches>{count}</TotalMatches>
<UpdateID>1</UpdateID>
</u:BrowseResponse>
</s:Body>
</s:Envelope>"#
                ),
                StatusCode::OK,
            )
        } else {
            (
                r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring></s:Fault></s:Body></s:Envelope>"#.to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        };

        return Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "text/xml; charset=utf-8")
            .body(resp)
            .unwrap();
    }

    Response::builder()
        .header(header::CONTENT_TYPE, "text/xml; charset=utf-8")
        .body(CONTENT_DIRECTORY_XML.to_string())
        .unwrap()
}
