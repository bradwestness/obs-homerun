# OBS HomeRun ⚾📺

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Container Image](https://img.shields.io/badge/Container-ghcr.io-blue?logo=docker)](https://github.com/bradwestness/obs-homerun/pkgs/container/obs-homerun)
[![Multi-Arch](https://img.shields.io/badge/Platforms-linux%2Famd64%20%7C%20linux%2Farm64-lightgrey)](#)

> **Turn your OBS Studio stream into a virtual HDTV tuner for Smart TVs.**  
> Zero TV apps to install. No typing IP addresses in a TV web browser. Works natively across modern Smart TVs (Samsung Tizen, LG webOS, Sony Bravia / Google TV, Roku, etc.).

---

## 🎯 The Problem

Streaming your PC desktop or gaming session to a living room TV over your local network is notoriously frustrating:
- **No TV Apps Needed:** You shouldn't have to sideload unapproved apps, renew expiring developer mode certificates on your TV OS, or buy an external streaming box.
- **Superior to Wireless Casting & Screen Projection:** Standard wireless mirroring protocols (such as Miracast, AirPlay, or Windows Wireless Display) rely on peer-to-peer Wi-Fi connections that suffer from aggressive compression artifacts, blurry text, dropped frames, and random disconnects. They also require keeping a source machine open, awake, and preventing sleep timeouts. OBS HomeRun routes directly over your local network (including wired Ethernet), delivering a stable broadcast-grade stream with higher bitrates, crisp visuals, and zero RF drops.
- **No Clunky TV Browsers:** Nobody wants to type `http://192.168.1.xxx:port` with a TV remote.
- **Generic DLNA Servers Fail on Live Feeds:** Media servers like Universal Media Server (UMS), Plex, or Jellyfin are designed for finished movie files on disk. When fed a live OBS feed, they crash with buffer overruns or try to perform byte-range seeks, causing playback to freeze after a few seconds or spin the loading wheel indefinitely.

## 💡 The Solution

**OBS HomeRun** packages a complete live broadcast bridge into a tiny, single container (~60 MB):

```mermaid
flowchart LR
    OBS["🖥️ OBS Studio<br/>(RTMP Stream)"] -->|:1935| MTX["MediaMTX<br/>(Ingest Engine)"]
    MTX -->|RTSP :8554| BROADCASTER["OBS HomeRun Broadcaster<br/>(SSDP + DLNA + ATSC Remuxer)"]
    BROADCASTER -->|SSDP Multicast :1900| TV["📺 Smart TV<br/>(DLNA / UPnP Tuner)"]
    TV -->|HTTP GET :5004<br/>MPEG-TS Live Broadcast| BROADCASTER
```

1. **Native TV Tuner Emulation:** Announces itself over SSDP (`239.255.255.250:1900`) using the UPnP/DLNA virtual tuner profile.
2. **Instant Native Discovery:** Your TV automatically sees your PC in its **Inputs / Home Dashboard / Devices** list as a TV tuner (Channel 1.1).
3. **Mid-Stream Decoder Lock:** Injects Sequence Parameter Set (SPS) and Picture Parameter Set (PPS) headers inline into every keyframe (`dump_extra`) so the TV's hardware decoder locks on immediately with zero dropped frames.
4. **Universal Broadcast Audio:** Converts audio to ATSC-standard Dolby Digital AC-3 on the fly.
5. **Jitter-Free Playback:** Maintains an internal ~3-second clock cushion (`muxdelay`/`muxpreload`) and expanded TCP buffer so local Wi-Fi micro-stutters never trigger a spinning wheel.
6. **Zero GPU Overhead:** Video is remuxed using stream-copy (0% GPU encode compute taken away from your games, creative apps, or productivity work).

---

## 🚀 Quick Start

> **Important:** The container **must** use host networking (`--net=host` or `network_mode: host`) so SSDP multicast discovery packets (`239.255.255.250:1900`) can reach your local subnet.
> - **Linux & NAS:** Native host networking works out-of-the-box.
> - **Windows & macOS:** Because standard Docker Desktop runs in a virtual machine that isolates multicast by default, see the [💻 Host OS Setup Guide](docs/host-os-setup.md) to run natively or use WSL2 mirrored networking.

### Option A: Podman Quadlet (Recommended for Bazzite / Fedora Silverblue / SteamOS)

Save as `~/.config/containers/systemd/obs-homerun.container`:

```ini
[Unit]
Description=OBS HomeRun - Virtual HDTV Tuner
After=network-online.target
Wants=network-online.target

[Container]
Image=ghcr.io/bradwestness/obs-homerun:latest
ContainerName=obs-homerun
Network=host
Pull=newer
AutoUpdate=registry
Environment="FRIENDLY_NAME=OBS HomeRun"
Environment=CHANNEL_NUMBER=1.1
Environment=BUFFER_SECONDS=3.0

[Install]
WantedBy=default.target
```

Reload and start:
```bash
systemctl --user daemon-reload
systemctl --user start obs-homerun.service
```

---

### Option B: Docker Compose / Synology & Network-Attached Storage (NAS)

Running OBS HomeRun on an always-on NAS (Synology Container Manager, TrueNAS SCALE, Unraid, QNAP, or any Linux server) is ideal: the broadcaster sits idle 24/7 on your home network without needing any containers or services running on your gaming/workstation PC.

Save as `docker-compose.yml`:

```yaml
services:
  obs-homerun:
    image: ghcr.io/bradwestness/obs-homerun:latest
    container_name: obs-homerun
    network_mode: host
    restart: unless-stopped
    environment:
      - FRIENDLY_NAME=OBS HomeRun
      - CHANNEL_NUMBER=1.1
      - BUFFER_SECONDS=3.0
```

Run:
```bash
docker compose up -d
```

> 💡 **Synology Container Manager:** Create a new project in Container Manager, upload or paste the `docker-compose.yml` above, and ensure the network is set to **host** (`network_mode: host`).

---

### Option C: CLI One-Liner

```bash
# Podman
podman run -d --net=host --name obs-homerun ghcr.io/bradwestness/obs-homerun:latest

# Docker
docker run -d --net=host --name obs-homerun ghcr.io/bradwestness/obs-homerun:latest
```

---

## 🎥 OBS Studio Configuration

### 1. Stream Settings
In OBS Studio $\rightarrow$ **Settings** $\rightarrow$ **Stream**:
- **Service:** Custom...
- **Server:**
  - If running on the **same PC**: `rtmp://localhost:1935/live`
  - If running on a **NAS or home server**: `rtmp://<NAS-IP>:1935/live` (e.g. `rtmp://192.168.1.50:1935/live`)
- **Stream Key:** `stream`

### 2. Output Settings (NVENC / QuickSync / AMF / x264)
In OBS Studio $\rightarrow$ **Settings** $\rightarrow$ **Output** (Output Mode: **Advanced** $\rightarrow$ **Streaming** tab):
- **Rate Control:** `CBR`
- **Bitrate:** `6000 Kbps` (or 8000–12000 Kbps if on 5GHz Wi-Fi / Ethernet)
- **Keyframe Interval:** `1 s` *(Recommended: gives frequent recovery points)*
- **Preset:** P5 / Medium or High Quality
- **Tuning:** High Quality (`hq`)
- **Max B-frames:** `0`

> 💡 **Canvas Framing Tip:** If the edges of your desktop appear clipped on your TV, your capture source scale might be zoomed in. In OBS, click your display capture source in the **Sources** dock and press **`Ctrl + R`** (Reset Transform) or **`Ctrl + F`** (Fit to Screen) to snap it cleanly to your canvas.

> 📖 **Comprehensive Guides:**
> - [💻 **Host OS Setup Guide (Linux, Windows, macOS)**](docs/host-os-setup.md): Native services, Windows Defender/PowerShell, macOS Homebrew/launchd, and Linux Quadlets.
> - [🎥 **Dedicated OBS Studio Configuration Guide**](docs/obs-configuration.md): Hardware encoder recipes (NVENC / AMF / QSV / x264), ultrawide aspect ratios, 5.1 surround sound, and profile management.
> - [🌐 **Network Setup & Troubleshooting Guide**](docs/network-and-troubleshooting.md): Firewall rules, router IGMP/multicast settings, and troubleshooting common streaming issues.

---

## 📺 Watching on Your Smart TV

1. Click **Start Streaming** in OBS Studio.
2. Turn on your Smart TV:
   - **Samsung Tizen:** Open **Connected Devices / Sources** $\rightarrow$ Select **OBS HomeRun** $\rightarrow$ Select Channel **1.1**.
   - **LG webOS:** Press **Source / Inputs** or open **Home Dashboard** $\rightarrow$ Select **OBS HomeRun** under Storage/Media Devices $\rightarrow$ Select Channel **1.1**.
   - **Sony Bravia / Android TV / Google TV:** Open the built-in **Media Player** app $\rightarrow$ Select **OBS HomeRun** under Servers $\rightarrow$ Select Channel **1.1**.
   - **Roku TV:** Open **Roku Media Player** $\rightarrow$ Select **Video** $\rightarrow$ Select **OBS HomeRun** $\rightarrow$ Select Channel **1.1**.
   - **Other DLNA Smart TVs / Media Streamers:** Open your TV's native media browser or input list $\rightarrow$ Select **OBS HomeRun**.
3. The stream will lock on within 1–2 seconds with crystal-clear video and audio!

---

## ⚙️ Configuration / Environment Variables

All settings are optional and have sensible defaults:

| Variable | Default | Description |
| :--- | :--- | :--- |
| `FRIENDLY_NAME` | `OBS HomeRun` | Device name shown in the TV Inputs / DLNA list |
| `CHANNEL_NUMBER` | `1.1` | Virtual ATSC channel number |
| `BUFFER_SECONDS` | `3.0` | Jitter cushion buffer duration in seconds (eliminates Wi-Fi stutter) |
| `AUDIO_CODEC` | `ac3` | Audio codec (`ac3` for native ATSC Dolby Digital, or `copy` for passthrough) |
| `AUDIO_BITRATE` | `384k` | Audio bitrate for AC-3 transcode |
| `HTTP_PORT` | `5004` | HTTP port for DLNA & virtual tuner streaming |
| `HOST_IP` | *Auto-detected* | Outbound LAN IP (auto-detected from default gateway if omitted) |
| `RTSP_SOURCE` | `rtsp://127.0.0.1:8554/live/stream` | Internal RTSP feed ingested by MediaMTX |

---

## 🛠️ How It Works Technically

Traditional media servers fail when streaming live desktop video to Smart TVs because:
- **VOD vs Live:** TVs expect live streams to omit `Content-Length` and declare UPnP class `object.item.videoItem.videoBroadcast` with `DLNA.ORG_OP=00` (no seeks).
- **Extradata Missing in Mid-Stream Connections:** Hardware H.264 decoders in modern TVs need SPS and PPS parameter sets at stream initialization. OBS NVENC typically sends these once at stream start. Mid-stream TV connections miss this data unless injected into IDR keyframes.
- **Clock Drift:** Live network streams without demux-decode clock offsets (`muxdelay`/`muxpreload`) leave the TV decoder buffer running at empty ($0\text{s}$ cushion), meaning any brief Wi-Fi ping spike triggers an immediate loading spinner.

OBS HomeRun bridges MediaMTX and FFmpeg bitstream filters to solve all three issues transparently with negligible CPU usage ($< 0.5\%$).

---

## 💤 Resource Usage & On-Demand Execution

OBS HomeRun is designed to run 24/7 as a background service without wasting system resources:

- **Idle (No OBS stream & No TV tuned in):**
  - Consumes **~14 MB RAM** total (Rust engine + MediaMTX).
  - Uses **0.0% CPU** and **0% GPU** (passively listening for SSDP discovery and incoming connections).
  - FFmpeg is **not running at all**.
- **OBS Streaming, TV Not Watching:**
  - MediaMTX receives your stream into an internal lightweight socket buffer.
  - FFmpeg is **still not running**—zero transcoding compute is consumed until a TV actively requests the feed.
- **TV Actively Watching:**
  - FFmpeg is spawned on-demand as a child process.
  - Video is remuxed using **stream-copy** (`-c:v copy`), meaning zero GPU encoding power is stolen from your games, creative apps, or productivity work.
- **TV Turns Off / Changes Inputs:**
  - The HTTP connection terminates and FFmpeg is immediately killed (`SIGKILL`), dropping resource usage back to zero.

---

## 🛑 What OBS HomeRun Is (and What It Isn't)

### What It Is:
- **A Zero-Configuration Live Broadcast Bridge:** Emulates an over-the-air (OTA) digital HDTV tuner over DLNA/UPnP and SSDP.
- **Instant Display Mirroring:** Designed to let you sit on your couch, turn on your Smart TV, and watch your PC or game stream immediately with zero TV apps or sideloading required.
- **Ultra-Lightweight (~14 MB Idle RAM):** Written in Rust with Tokio and MediaMTX to sit idle in the background 24/7 without consuming CPU, GPU, or memory.

### What It Is Not:
- **Not a Full DVR or Media Center:** OBS HomeRun has **no server-side recording, disk buffering, or persistent media library**.
- **No Server-Side Rewind / Time-Shifting:** Just like a physical television antenna tuner, OBS HomeRun outputs a live infinite broadcast stream (`DLNA.ORG_OP=00`). In DLNA, advertising seekability (`DLNA.ORG_OP=01`) on a live stream requires fixed file lengths and byte-range requests; doing so causes TVs to freeze or display endless loading spinners.

### Need Full DVR, Scheduled Recording, or Multi-Room Time-Shifting?
If you are looking for scheduled DVR recordings, multi-hour pause/rewind buffers, or electronic program guides (EPG), you can connect heavier full-DVR suites to OBS HomeRun's HTTP tuner stream (`http://<HOST-IP>:5004/auto/v1.1`):
- [Channels DVR](https://getchannels.com/) — Premier whole-home live TV & DVR suite with custom M3U/HDHomeRun tuner support, guide data, commercial skipping, and multi-room time-shifting.
- [Plex Live TV & DVR](https://www.plex.tv/tv/) — Integrates virtual tuner feeds into your Plex media server for scheduled DVR passes and remote streaming.
- [Kodi](https://kodi.tv/) — Open-source home theater software with PVR plugins (such as IPTV Simple Client or Tvheadend) for client-side timeshifting and recording.
- [Jellyfin](https://jellyfin.org/) — Free, open-source media system featuring built-in Live TV and DVR recording functionality.

*(Note: If your Smart TV supports local USB recording/time-shifting—such as plugging a USB drive into the TV for native live pause—your TV's built-in tuner software may handle client-side pausing directly).*

---

## 📄 License

MIT © [Brad Westness](LICENSE)
