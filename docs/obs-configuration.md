# 🎥 OBS Studio Configuration Guide

This guide covers optimal OBS Studio configurations for streaming your PC desktop or gameplay to a living room Smart TV with **OBS HomeRun**.

---

## 📋 Table of Contents
1. [Stream Destination Settings](#1-stream-destination-settings)
2. [Encoder Tuning by Hardware](#2-encoder-tuning-by-hardware)
   - [NVIDIA NVENC (GeForce RTX / GTX)](#nvidia-nvenc-recommended)
   - [AMD AMF / VA-API (Radeon RX)](#amd-amf--va-api)
   - [Intel QuickSync (Arc / Iris Xe / Core iGPU)](#intel-quicksync-qsv)
   - [Software x264 (CPU Only)](#software-x264-cpu-only)
3. [Resolution & Aspect Ratio Management](#3-resolution--aspect-ratio-management)
   - [Standard 16:9 Displays](#standard-169-displays)
   - [Ultrawide Monitors (21:9 & 32:9)](#ultrawide-monitors-219--329)
4. [Audio Configuration & 5.1 Surround Sound](#4-audio-configuration--51-surround-sound)
5. [Network & Bitrate Recommendations](#5-network--bitrate-recommendations)
6. [Workflow Tip: Dedicated OBS Streaming Profile](#6-workflow-tip-dedicated-obs-streaming-profile)

---

## 1. Stream Destination Settings

In OBS Studio, open **Settings** $\rightarrow$ **Stream**:

| Setting | Value | Notes |
| :--- | :--- | :--- |
| **Service** | `Custom...` | Do not select Twitch/YouTube |
| **Server** | `rtmp://localhost:1935/live` *(Local)*<br/>`rtmp://<NAS-IP>:1935/live` *(NAS / Server)* | Use localhost if running locally, or your NAS/server IP (e.g. `192.168.1.50`) |
| **Stream Key** | `stream` | Connects to `rtsp://127.0.0.1:8554/live/stream` |

> 💡 **NAS & Server Deployments:** If `obs-homerun` is running on a Synology NAS, TrueNAS, Unraid, or server, enter your NAS LAN IP in the **Server** field (e.g. `rtmp://192.168.1.50:1935/live`). Your PC does not need any local services running.
>
> 💡 **Flatpak OBS (Linux) Note:** If you are running OBS as a Flatpak locally on the same host, `localhost` may resolve to the Flatpak sandbox. If connection fails, set **Server** to your machine's LAN IP (e.g. `rtmp://192.168.1.150:1935/live`) or `rtmp://host.containers.internal:1935/live`.

---

## 2. Encoder Tuning by Hardware

In OBS Studio $\rightarrow$ **Settings** $\rightarrow$ **Output**:
Set **Output Mode** to **Advanced**, then select the **Streaming** tab.

### NVIDIA NVENC (Recommended)

NVIDIA hardware encoding provides virtually zero performance impact on gameplay.

* **Video Codec:** `NVIDIA NVENC H.264` *(Do not use AV1 or HEVC for basic ATSC DLNA compatibility)*
* **Rate Control:** `CBR`
* **Bitrate:** `6000 Kbps` to `12000 Kbps` (see [Network Recommendations](#5-network--bitrate-recommendations))
* **Keyframe Interval:** `1 s`  
  *(⚠️ **Critical:** Setting this to 1s ensures the TV locks onto the feed in ~1 second and quickly recovers from Wi-Fi drops. Default 0s/auto is 2s or 5s).*
* **Preset:** `P5: Slow (Good Quality)` or `P6: Slower (Better Quality)`
* **Tuning:** `High Quality` (`hq`)
* **Multipass Mode:** `Two Passes (Quarter Resolution)`
* **Profile:** `high`
* **Lookahead:** `Unchecked` (can introduce B-frame jitter)
* **Psycho Visual Tuning:** `Checked`
* **GPU:** `0` (or your secondary streaming GPU ID if multi-GPU)
* **Max B-frames:** `0`  
  *(⚠️ **Critical:** Zero B-frames ensures the simplest decoding pipeline for Smart TV hardware decoders).*

---

### AMD AMF / VA-API

For AMD Radeon graphics cards on Windows (AMF) or Linux (VA-API / Mesa):

* **Video Codec:** `AMD HW H.264` (Windows) or `FFmpeg VAAPI` / `AMD AMF H.264` (Linux)
* **Rate Control:** `CBR`
* **Bitrate:** `6000 Kbps` – `10000 Kbps`
* **Keyframe Interval:** `1.00 s` (or 60 frames if 60 fps)
* **Preset:** `Quality`
* **Profile:** `high`
* **Max B-frames:** `0`

---

### Intel QuickSync (QSV)

For Intel Core integrated graphics or Intel Arc discrete GPUs:

* **Video Codec:** `QuickSync H.264`
* **Target Usage:** `TU4: Balanced` or `TU2: High Quality`
* **Rate Control:** `CBR`
* **Bitrate:** `6000 Kbps` – `10000 Kbps`
* **Keyframe Interval:** `1 s`
* **B-frames:** `0`
* **Profile:** `high`

---

### Software x264 (CPU Only)

If your system lacks a dedicated hardware encoder:

* **Video Codec:** `x264`
* **Rate Control:** `CBR`
* **Bitrate:** `4500 Kbps` – `6000 Kbps`
* **Keyframe Interval:** `1`
* **CPU Usage Preset:** `veryfast` or `superfast`
* **Profile:** `high`
* **Tune:** `zerolatency`
* **x264 Options:** `bframes=0`

---

## 3. Resolution & Aspect Ratio Management

Smart TVs natively expect standard **16:9** aspect ratios.

### Standard 16:9 Displays
In **Settings** $\rightarrow$ **Video**:
* **Base (Canvas) Resolution:** Match your primary monitor (`1920x1080`, `2560x1440`, or `3840x2160`).
* **Output (Scaled) Resolution:** Match your TV's resolution or network capability:
  * `1920x1080` (1080p60) — Flawless on virtually all 2.4GHz/5GHz Wi-Fi networks.
  * `2560x1440` (1440p60) — Crisp desktop text; downscaled cleanly by 4K TVs.
  * `3840x2160` (4K60) — Native 4K OLED; requires strong 5GHz Wi-Fi or Ethernet and bitrates $\ge 12000\text{ Kbps}$.
* **Downscale Filter:** `Lanczos (Sharpened scaling, 36 samples)`
* **Common FPS Values:** `60`

#### Quick Canvas Alignment Hotkeys:
If your desktop appears zoomed in or clipped on the TV:
1. Click your display capture source in the OBS preview window.
2. Press **`Ctrl + R`** to reset the transform.
3. Press **`Ctrl + F`** to cleanly fit it to the canvas.

---

### Ultrawide Monitors (21:9 & 32:9)

If your PC gaming monitor is ultrawide (`2560x1080`, `3440x1440`, or `5120x1440`), broadcasting directly will either letterbox (black bars top & bottom) or stretch on a 16:9 TV.

#### Option A: 16:9 Pillarbox Canvas (Recommended)
1. In OBS **Settings** $\rightarrow$ **Video**, set **Base (Canvas) Resolution** to a 16:9 resolution:
   - For a `3440x1440` monitor, set canvas to `2560x1440` or `3840x2160`.
2. Add your **Screen Capture** source.
3. Center the capture source:
   - You can scale it so the full ultrawide frame is visible with black bars on top and bottom (true cinema look).
   - Alternatively, center-crop the 16:9 active gameplay area.

#### Option B: Virtual 16:9 Display / Dummy Plug
If you want true native full-screen 16:9 rendering without black bars on the TV:
* Use a software virtual display (e.g. **Virtual Display Driver** on Windows, or **gamescope / wayland-virtual-output** on Linux).
* Set the virtual display to `3840x2160` (4K) or `2560x1440`, mirror or render games to it, and capture that display in OBS.

---

## 4. Audio Configuration & 5.1 Surround Sound

OBS HomeRun converts incoming stream audio to ATSC-standard **Dolby Digital AC-3** at 384 Kbps on the fly.

### Audio Sample Rate
In OBS **Settings** $\rightarrow$ **Audio**:
* **Sample Rate:** Set to **`48 kHz`** *(Broadcast standard; prevents clock sample rate resampling).*
* **Channels:**
  * **Stereo** (Standard 2.0).
  * **5.1 Surround:** OBS supports native 5.1 channel capture! If your PC is outputting 5.1 audio (games or media player), set **Channels** to **`5.1`**. OBS HomeRun will encode all 6 discrete channels into native Dolby Digital 5.1 for your TV soundbar or AV receiver!

---

## 5. Network & Bitrate Recommendations

Because OBS HomeRun streams over your local network rather than the public internet, you are limited only by your local Wi-Fi or Ethernet bandwidth:

| Connection Type | Recommended Resolution | Recommended Bitrate |
| :--- | :--- | :--- |
| **Ethernet (1 Gbps LAN)** | 4K UHD (3840x2160) @ 60fps | `15000` – `25000 Kbps` |
| **Wi-Fi 6 / 5GHz (Strong signal)** | 1440p or 4K @ 60fps | `10000` – `15000 Kbps` |
| **Wi-Fi 5 / Standard 5GHz** | 1080p @ 60fps | `6000` – `8000 Kbps` |
| **2.4 GHz Wi-Fi** | 1080p @ 60fps | `4000` – `5000 Kbps` |

> 💡 **Bitrate Tip:** Broadcast TV over ATSC typically runs at 12–19 Mbps. A bitrate of `8000–12000 Kbps` with NVENC provides virtually lossless desktop and 60fps gaming quality on a 4K OLED TV.

---

## 6. Workflow Tip: Dedicated OBS Streaming Profile

To avoid overwriting your Twitch or YouTube stream settings, create a dedicated profile in OBS:

1. Click **Profile** in the top menu $\rightarrow$ **New** $\rightarrow$ Name it **`OBS HomeRun - Living Room TV`**.
2. Click **Scene Collection** $\rightarrow$ **New** $\rightarrow$ Name it **`Living Room TV`**.
3. Apply the settings from this guide.

Now, whenever you want to chill on the couch and stream your PC to the TV, just switch to the **Living Room TV** profile with a single click!
