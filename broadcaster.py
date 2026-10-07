#!/usr/bin/env python3
"""
OBS HomeRun - Virtual HDTV / DLNA Live Stream Broadcaster
Disguises your OBS desktop stream as a native virtual HDTV tuner over UPnP/DLNA and SSDP.
"""

import sys
import os
import time
import socket
import struct
import threading
import subprocess
import uuid
from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
import email.utils


def get_default_host_ip():
    """Auto-detect the primary outbound LAN IP of the host."""
    env_ip = os.environ.get("HOST_IP")
    if env_ip and env_ip.strip():
        return env_ip.strip()
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"


# Configuration from Environment Variables
HOST_IP = get_default_host_ip()
HTTP_PORT = int(os.environ.get("HTTP_PORT", "5004"))
FRIENDLY_NAME = os.environ.get("FRIENDLY_NAME", "OBS HomeRun")
CHANNEL_NUMBER = os.environ.get("CHANNEL_NUMBER", "1.1")
RTSP_SOURCE = os.environ.get("RTSP_SOURCE", "rtsp://127.0.0.1:8554/live/stream")
BUFFER_SECONDS = float(os.environ.get("BUFFER_SECONDS", "3.0"))
AUDIO_CODEC = os.environ.get("AUDIO_CODEC", "ac3")
AUDIO_BITRATE = os.environ.get("AUDIO_BITRATE", "384k")

# Deterministic UUID based on namespace and friendly name
DEVICE_UUID = os.environ.get(
    "DEVICE_UUID",
    str(uuid.uuid5(uuid.NAMESPACE_DNS, f"obs-homerun.{FRIENDLY_NAME}"))
)

# Half the buffer duration goes to muxdelay, half to muxpreload
HALF_BUFFER = f"{max(0.5, BUFFER_SECONDS / 2.0):.1f}"

DEVICE_XML = f"""<?xml version="1.0" encoding="utf-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0" xmlns:dlna="urn:schemas-dlna-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <friendlyName>{FRIENDLY_NAME}</friendlyName>
    <presentationURL>/</presentationURL>
    <manufacturer>OBS HomeRun</manufacturer>
    <manufacturerURL>https://github.com/bradwestness/obs-homerun</manufacturerURL>
    <modelDescription>{FRIENDLY_NAME} Virtual HDTV Tuner</modelDescription>
    <modelName>OBS HomeRun</modelName>
    <modelNumber>HDTV-1.0</modelNumber>
    <modelURL>https://github.com/bradwestness/obs-homerun</modelURL>
    <serialNumber>107BDESK</serialNumber>
    <UDN>uuid:{DEVICE_UUID}</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <SCPDURL>/dms/ConnectionManager.xml</SCPDURL>
        <controlURL>http://{HOST_IP}:{HTTP_PORT}/dms/ConnectionManager</controlURL>
        <eventSubURL>http://{HOST_IP}:{HTTP_PORT}/dms/ConnectionManager</eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <SCPDURL>/dms/ContentDirectory.xml</SCPDURL>
        <controlURL>http://{HOST_IP}:{HTTP_PORT}/dms/ContentDirectory</controlURL>
        <eventSubURL>http://{HOST_IP}:{HTTP_PORT}/dms/ContentDirectory</eventSubURL>
      </service>
    </serviceList>
  </device>
</root>"""

CONNECTION_MANAGER_XML = """<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <actionList>
    <action>
      <name>GetProtocolInfo</name>
      <argumentList>
        <argument>
          <name>Source</name>
          <direction>out</direction>
          <relatedStateVariable>SourceProtocolInfo</relatedStateVariable>
        </argument>
        <argument>
          <name>Sink</name>
          <direction>out</direction>
          <relatedStateVariable>SinkProtocolInfo</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetCurrentConnectionIDs</name>
      <argumentList>
        <argument>
          <name>ConnectionIDs</name>
          <direction>out</direction>
          <relatedStateVariable>CurrentConnectionIDs</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="yes">
      <name>SourceProtocolInfo</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="yes">
      <name>SinkProtocolInfo</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="yes">
      <name>CurrentConnectionIDs</name>
      <dataType>string</dataType>
    </stateVariable>
  </serviceStateTable>
</scpd>"""

CONTENT_DIRECTORY_XML = """<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <actionList>
    <action>
      <name>Browse</name>
      <argumentList>
        <argument>
          <name>ObjectID</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_ObjectID</relatedStateVariable>
        </argument>
        <argument>
          <name>BrowseFlag</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_BrowseFlag</relatedStateVariable>
        </argument>
        <argument>
          <name>Filter</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Filter</relatedStateVariable>
        </argument>
        <argument>
          <name>StartingIndex</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Index</relatedStateVariable>
        </argument>
        <argument>
          <name>RequestedCount</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>SortCriteria</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_SortCriteria</relatedStateVariable>
        </argument>
        <argument>
          <name>Result</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Result</relatedStateVariable>
        </argument>
        <argument>
          <name>NumberReturned</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>TotalMatches</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>UpdateID</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_UpdateID</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetSearchCapabilities</name>
      <argumentList>
        <argument>
          <name>SearchCaps</name>
          <direction>out</direction>
          <relatedStateVariable>SearchCapabilities</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetSortCapabilities</name>
      <argumentList>
        <argument>
          <name>SortCaps</name>
          <direction>out</direction>
          <relatedStateVariable>SortCapabilities</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetSystemUpdateID</name>
      <argumentList>
        <argument>
          <name>Id</name>
          <direction>out</direction>
          <relatedStateVariable>SystemUpdateID</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_SortCriteria</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_UpdateID</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Filter</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Result</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Index</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_ObjectID</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>SortCapabilities</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>SearchCapabilities</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Count</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_BrowseFlag</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>BrowseMetadata</allowedValue>
        <allowedValue>BrowseDirectChildren</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="yes">
      <name>SystemUpdateID</name>
      <dataType>ui4</dataType>
    </stateVariable>
  </serviceStateTable>
</scpd>"""

DIDL_LITE_ITEM = f"""&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/"&gt;&lt;item id="v1" parentID="0" restricted="1"&gt;&lt;dc:title&gt;{FRIENDLY_NAME}&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;{CHANNEL_NUMBER}&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;{FRIENDLY_NAME}&lt;/upnp:channelName&gt;&lt;res protocolInfo="http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/mp2t:*"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;"""

DIDL_LITE_CONTAINERS = f"""&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/"&gt;&lt;container id="Channels" parentID="0" restricted="1"&gt;&lt;dc:title&gt;Channels&lt;/dc:title&gt;&lt;upnp:class&gt;object.container&lt;/upnp:class&gt;&lt;dlna:containerType&gt;Tuner_1_0&lt;/dlna:containerType&gt;&lt;/container&gt;&lt;item id="v1" parentID="0" restricted="1"&gt;&lt;dc:title&gt;{FRIENDLY_NAME}&lt;/dc:title&gt;&lt;upnp:class&gt;object.item.videoItem.videoBroadcast&lt;/upnp:class&gt;&lt;upnp:channelNr&gt;{CHANNEL_NUMBER}&lt;/upnp:channelNr&gt;&lt;upnp:channelName&gt;{FRIENDLY_NAME}&lt;/upnp:channelName&gt;&lt;res protocolInfo="http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/mp2t:*"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;res protocolInfo="http-get:*:video/vnd.dlna.mpeg-tts:DLNA.ORG_PN=AVC_TS_NA_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"&gt;http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}&lt;/res&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;"""


class ThreadedHTTPServer(ThreadingMixIn, HTTPServer):
    daemon_threads = True


class BroadcasterHTTPHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def log_message(self, format, *args):
        try:
            msg = format % args
        except Exception:
            msg = str(format)
        sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {self.address_string()} - {msg}\n")

    def do_HEAD(self):
        if self.path.startswith("/auto/") or self.path.startswith("/live") or self.path.startswith("/stream"):
            self.send_response(200)
            self.send_header("Server", "Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0")
            self.send_header("Connection", "close")
            self.send_header("Content-Type", "video/mpeg")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("transferMode.dlna.org", "Streaming")
            self.send_header("contentFeatures.dlna.org", "DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000")
            self.end_headers()
        else:
            self.send_response(200)
            self.end_headers()

    def do_GET(self):
        if self.path == "/discover.json":
            body = (
                f'{{"FriendlyName":"{FRIENDLY_NAME}",'
                f'"ModelNumber":"HDTV-1.0",'
                f'"FirmwareName":"v_atsc_tuner",'
                f'"FirmwareVersion":"20260101",'
                f'"DeviceID":"107BDESK",'
                f'"DeviceAuth":"desktop",'
                f'"BaseURL":"http://{HOST_IP}:{HTTP_PORT}",'
                f'"LineupURL":"http://{HOST_IP}:{HTTP_PORT}/lineup.json",'
                f'"TunerCount":2}}'
            ).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(body)

        elif self.path == "/lineup.json":
            body = f'[{{"GuideNumber":"{CHANNEL_NUMBER}","GuideName":"{FRIENDLY_NAME}","URL":"http://{HOST_IP}:{HTTP_PORT}/auto/v{CHANNEL_NUMBER}"}}]'.encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(body)

        elif self.path == "/dms/device.xml":
            body = DEVICE_XML.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/xml; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        elif self.path == "/dms/ConnectionManager.xml":
            body = CONNECTION_MANAGER_XML.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/xml; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        elif self.path == "/dms/ContentDirectory.xml":
            body = CONTENT_DIRECTORY_XML.encode("utf-8")
            self.send_response(200)
            self.send_header("Content-Type", "text/xml; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        elif self.path.startswith("/auto/") or self.path.startswith("/live") or self.path.startswith("/stream"):
            client_ip = self.client_address[0]
            sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [Stream] CONNECT from {client_ip} for {self.path}\n")
            for h, v in self.headers.items():
                sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [Header] {h}: {v}\n")
            sys.stderr.flush()

            self.send_response(200)
            self.send_header("Server", "Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0")
            self.send_header("Connection", "close")
            self.send_header("Content-Type", "video/mpeg")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.send_header("transferMode.dlna.org", "Streaming")
            self.send_header("contentFeatures.dlna.org", "DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000")
            self.end_headers()

            # Increase TCP send buffer to 2MB to smooth over network jitter
            try:
                self.connection.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 2 * 1024 * 1024)
            except Exception:
                pass

            # Audio configuration
            audio_args = ["-c:a", "copy"] if AUDIO_CODEC == "copy" else ["-c:a", AUDIO_CODEC, "-b:a", AUDIO_BITRATE]

            # Remux RTSP to broadcast MPEG-TS with inline SPS/PPS and buffer cushion
            ffmpeg_cmd = [
                "ffmpeg",
                "-hide_banner",
                "-loglevel", "warning",
                "-rtsp_transport", "tcp",
                "-analyzeduration", "500000",
                "-probesize", "1000000",
                "-i", RTSP_SOURCE,
                "-c:v", "copy",
                "-bsf:v", "dump_extra",
                *audio_args,
                "-muxdelay", HALF_BUFFER,
                "-muxpreload", HALF_BUFFER,
                "-pat_period", "0.1",
                "-mpegts_flags", "+initial_discontinuity+resend_headers",
                "-f", "mpegts",
                "pipe:1"
            ]

            proc = None
            bytes_sent = 0
            start_t = time.time()
            try:
                proc = subprocess.Popen(
                    ffmpeg_cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.DEVNULL,
                    bufsize=65536
                )
                while True:
                    chunk = proc.stdout.read(65536)
                    if not chunk:
                        break
                    self.wfile.write(chunk)
                    self.wfile.flush()
                    bytes_sent += len(chunk)
            except (BrokenPipeError, ConnectionResetError) as e:
                dur = time.time() - start_t
                sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [Stream] DISCONNECT from {client_ip}: sent {bytes_sent} bytes over {dur:.1f}s ({e})\n")
            except Exception as e:
                sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [Stream] ERROR from {client_ip}: {e}\n")
            finally:
                if proc:
                    try:
                        proc.terminate()
                        proc.wait(timeout=0.5)
                    except Exception:
                        try:
                            proc.kill()
                            proc.wait(timeout=0.5)
                        except Exception:
                            pass
                    try:
                        if proc.stdout:
                            proc.stdout.close()
                    except Exception:
                        pass
                sys.stderr.flush()

        else:
            self.send_response(404)
            self.end_headers()
            self.wfile.write(b"Not Found")

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        data = self.rfile.read(length).decode("utf-8", errors="ignore")

        if "ConnectionManager" in self.path:
            if "GetProtocolInfo" in data:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetProtocolInfoResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<Source>http-get:*:video/mpeg:DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000,http-get:*:video/mpeg:*,http-get:*:video/mp2t:*,http-get:*:video/vnd.dlna.mpeg-tts:*</Source>
<Sink></Sink>
</u:GetProtocolInfoResponse>
</s:Body>
</s:Envelope>"""
            else:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetCurrentConnectionIDsResponse xmlns:u="urn:schemas-upnp-org:service:ConnectionManager:1">
<ConnectionIDs>0</ConnectionIDs>
</u:GetCurrentConnectionIDsResponse>
</s:Body>
</s:Envelope>"""
        elif "ContentDirectory" in self.path:
            if "GetSystemUpdateID" in data:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetSystemUpdateIDResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<Id>1</Id>
</u:GetSystemUpdateIDResponse>
</s:Body>
</s:Envelope>"""
            elif "GetSortCapabilities" in data:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetSortCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<SortCaps></SortCaps>
</u:GetSortCapabilitiesResponse>
</s:Body>
</s:Envelope>"""
            elif "GetSearchCapabilities" in data:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:GetSearchCapabilitiesResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<SearchCaps></SearchCaps>
</u:GetSearchCapabilitiesResponse>
</s:Body>
</s:Envelope>"""
            elif "Browse" in data:
                if 'ObjectID>0</' in data or 'ObjectID&gt;0&lt;/' in data:
                    result = DIDL_LITE_CONTAINERS
                    count = 2
                else:
                    result = DIDL_LITE_ITEM
                    count = 1

                resp = f"""<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
<s:Body>
<u:BrowseResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
<Result>{result}</Result>
<NumberReturned>{count}</NumberReturned>
<TotalMatches>{count}</TotalMatches>
<UpdateID>1</UpdateID>
</u:BrowseResponse>
</s:Body>
</s:Envelope>"""
            else:
                resp = """<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><s:Fault><faultcode>s:Client</faultcode><faultstring>UPnPError</faultstring></s:Fault></s:Body></s:Envelope>"""
        else:
            resp = ""

        body = resp.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/xml; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


def run_ssdp():
    MULTICAST_GROUP = "239.255.255.250"
    SSDP_PORT = 1900

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("", SSDP_PORT))

    mreq = struct.pack("4sl", socket.inet_aton(MULTICAST_GROUP), socket.INADDR_ANY)
    sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)

    targets = [
        "upnp:rootdevice",
        f"uuid:{DEVICE_UUID}",
        "urn:schemas-upnp-org:device:MediaServer:1",
        "urn:schemas-upnp-org:service:ContentDirectory:1",
        "urn:schemas-upnp-org:service:ConnectionManager:1",
    ]

    def send_notify():
        notify_sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
        notify_sock.setsockopt(socket.IPPROTO_IP, socket.IP_MULTICAST_TTL, 4)
        for t in targets:
            usn = f"uuid:{DEVICE_UUID}::{t}" if t != f"uuid:{DEVICE_UUID}" else f"uuid:{DEVICE_UUID}"
            msg = (
                f"NOTIFY * HTTP/1.1\r\n"
                f"HOST: {MULTICAST_GROUP}:{SSDP_PORT}\r\n"
                f"NT: {t}\r\n"
                f"NTS: ssdp:alive\r\n"
                f"LOCATION: http://{HOST_IP}:{HTTP_PORT}/dms/device.xml\r\n"
                f"USN: {usn}\r\n"
                f"CACHE-CONTROL: max-age=1800\r\n"
                f"SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n\r\n"
            )
            try:
                notify_sock.sendto(msg.encode(), (MULTICAST_GROUP, SSDP_PORT))
            except Exception:
                pass
        notify_sock.close()

    # Initial announcement
    send_notify()

    # Periodic announcement thread every 60s
    def periodic():
        while True:
            time.sleep(60)
            send_notify()

    threading.Thread(target=periodic, daemon=True).start()

    sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [SSDP] Listening on {MULTICAST_GROUP}:{SSDP_PORT}...\n")
    sys.stderr.flush()

    while True:
        try:
            data, addr = sock.recvfrom(2048)
            text = data.decode("utf-8", errors="ignore")
            if "M-SEARCH" in text:
                st = None
                for line in text.split("\r\n"):
                    if line.upper().startswith("ST:"):
                        st = line.split(":", 1)[1].strip()
                        break

                if st:
                    matched = []
                    if st == "ssdp:all":
                        matched = targets
                    elif st in targets:
                        matched = [st]

                    for m in matched:
                        usn = f"uuid:{DEVICE_UUID}::{m}" if m != f"uuid:{DEVICE_UUID}" else f"uuid:{DEVICE_UUID}"
                        resp = (
                            f"HTTP/1.1 200 OK\r\n"
                            f"CACHE-CONTROL: max-age=1800\r\n"
                            f"DATE: {email.utils.formatdate(usegmt=True)}\r\n"
                            f"EXT:\r\n"
                            f"LOCATION: http://{HOST_IP}:{HTTP_PORT}/dms/device.xml\r\n"
                            f"SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n"
                            f"ST: {m}\r\n"
                            f"USN: {usn}\r\n\r\n"
                        )
                        sock.sendto(resp.encode(), addr)
        except Exception:
            pass


def main():
    sys.stderr.write(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] [Broadcaster] Starting '{FRIENDLY_NAME}' on {HOST_IP}:{HTTP_PORT}...\n")
    sys.stderr.flush()

    server = ThreadedHTTPServer((HOST_IP, HTTP_PORT), BroadcasterHTTPHandler)

    ssdp_thread = threading.Thread(target=run_ssdp, daemon=True)
    ssdp_thread.start()

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        sys.stderr.write("\nShutting down.\n")


if __name__ == "__main__":
    main()
