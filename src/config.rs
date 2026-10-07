use md5::{Digest, Md5};
use std::net::UdpSocket;

#[derive(Clone, Debug)]
pub struct Config {
    pub friendly_name: String,
    pub channel_number: String,
    pub host_ip: String,
    pub http_port: u16,
    pub rtsp_source: String,
    pub buffer_sec: f64,
    pub half_buffer: String,
    pub audio_codec: String,
    pub audio_bitrate: String,
    pub device_uuid: String,
}

#[must_use]
pub fn get_default_host_ip() -> String {
    if let Ok(env) = std::env::var("HOST_IP") {
        let trimmed = env.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                return local_addr.ip().to_string();
            }
        }
    }

    "127.0.0.1".to_string()
}

#[must_use]
pub fn generate_uuid(name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("obs-homerun.{name}").as_bytes());
    let mut h = hasher.finalize();

    // RFC 4122 version 3 and variant
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7], h[8], h[9], h[10], h[11], h[12], h[13], h[14], h[15]
    )
}

impl Config {
    #[must_use]
    pub fn from_env() -> Self {
        let friendly_name =
            std::env::var("FRIENDLY_NAME").unwrap_or_else(|_| "OBS HomeRun".to_string());
        let channel_number = std::env::var("CHANNEL_NUMBER").unwrap_or_else(|_| "1.1".to_string());
        let http_port = std::env::var("HTTP_PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(5004);
        let host_ip = get_default_host_ip();
        let rtsp_source = std::env::var("RTSP_SOURCE")
            .unwrap_or_else(|_| "rtsp://127.0.0.1:8554/live/stream".to_string());

        let buffer_sec = std::env::var("BUFFER_SECONDS")
            .ok()
            .and_then(|b| b.parse::<f64>().ok())
            .unwrap_or(3.0);
        let mut half = buffer_sec / 2.0;
        if half < 0.5 {
            half = 0.5;
        }
        let half_buffer = format!("{half:.1}");

        let audio_codec = std::env::var("AUDIO_CODEC").unwrap_or_else(|_| "ac3".to_string());
        let audio_bitrate = std::env::var("AUDIO_BITRATE").unwrap_or_else(|_| "384k".to_string());

        let device_uuid =
            std::env::var("DEVICE_UUID").unwrap_or_else(|_| generate_uuid(&friendly_name));

        Self {
            friendly_name,
            channel_number,
            host_ip,
            http_port,
            rtsp_source,
            buffer_sec,
            half_buffer,
            audio_codec,
            audio_bitrate,
            device_uuid,
        }
    }

    #[must_use]
    pub fn device_xml(&self) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:dlna="urn:schemas-dlna-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <friendlyName>{name}</friendlyName>
    <presentationURL>/</presentationURL>
    <manufacturer>OBS HomeRun</manufacturer>
    <manufacturerURL>https://github.com/bradwestness/obs-homerun</manufacturerURL>
    <modelDescription>{name} Virtual HDTV Tuner</modelDescription>
    <modelName>OBS HomeRun</modelName>
    <modelNumber>HDTV-1.0</modelNumber>
    <modelURL>https://github.com/bradwestness/obs-homerun</modelURL>
    <serialNumber>107BDESK</serialNumber>
    <UDN>uuid:{uuid}</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <SCPDURL>/dms/ConnectionManager.xml</SCPDURL>
        <controlURL>http://{host}:{port}/dms/ConnectionManager</controlURL>
        <eventSubURL>http://{host}:{port}/dms/ConnectionManager</eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <SCPDURL>/dms/ContentDirectory.xml</SCPDURL>
        <controlURL>http://{host}:{port}/dms/ContentDirectory</controlURL>
        <eventSubURL>http://{host}:{port}/dms/ContentDirectory</eventSubURL>
      </service>
    </serviceList>
  </device>
</root>"#,
            name = self.friendly_name,
            uuid = self.device_uuid,
            host = self.host_ip,
            port = self.http_port
        )
    }

    #[must_use]
    pub fn discover_json(&self) -> String {
        serde_json::json!({
            "FriendlyName": self.friendly_name,
            "ModelNumber": "HDTV-1.0",
            "FirmwareName": "v_atsc_tuner",
            "FirmwareVersion": "20260101",
            "DeviceID": "107BDESK",
            "DeviceAuth": "desktop",
            "BaseURL": format!("http://{}:{}", self.host_ip, self.http_port),
            "LineupURL": format!("http://{}:{}/lineup.json", self.host_ip, self.http_port),
            "TunerCount": 2
        })
        .to_string()
    }

    #[must_use]
    pub fn lineup_json(&self) -> String {
        serde_json::json!([
            {
                "GuideNumber": self.channel_number,
                "GuideName": self.friendly_name,
                "URL": format!("http://{}:{}/auto/v{}", self.host_ip, self.http_port, self.channel_number)
            }
        ])
        .to_string()
    }

    #[must_use]
    pub fn didl_item(&self) -> String {
        format!(
            "&lt;DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\" xmlns:dlna=\"urn:schemas-dlna-org:metadata-1-0/\"&gt;&lt;item id=\"v1\" parentID=\"0\" restricted=\"1\"&gt;&lt;dc:title&gt;{name}&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;{ch}&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;{name}&lt;/upnp:channelName&gt;&lt;res protocolInfo=\"http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;res protocolInfo=\"http-get:*:video/mp2t:*\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;res protocolInfo=\"http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;",
            name = self.friendly_name,
            ch = self.channel_number,
            host = self.host_ip,
            port = self.http_port
        )
    }

    #[must_use]
    pub fn didl_containers(&self) -> String {
        format!(
            "&lt;DIDL-Lite xmlns=\"urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:upnp=\"urn:schemas-upnp-org:metadata-1-0/upnp/\" xmlns:dlna=\"urn:schemas-dlna-org:metadata-1-0/\"&gt;&lt;container id=\"Channels\" parentID=\"0\" restricted=\"1\"&gt;&lt;dc:title&gt;Channels&lt;/dc:title&gt;&lt;upnp:class&gt;object.container&lt;/upnp:class&gt;&lt;dlna:containerType&gt;Tuner_1_0&lt;/dlna:containerType&gt;&lt;/container&gt;&lt;item id=\"v1\" parentID=\"0\" restricted=\"1\"&gt;&lt;dc:title&gt;{name}&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;{ch}&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;{name}&lt;/upnp:channelName&gt;&lt;res protocolInfo=\"http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;res protocolInfo=\"http-get:*:video/mp2t:*\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;res protocolInfo=\"http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000\"&gt;http://{host}:{port}/auto/v{ch}&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;",
            name = self.friendly_name,
            ch = self.channel_number,
            host = self.host_ip,
            port = self.http_port
        )
    }
}
