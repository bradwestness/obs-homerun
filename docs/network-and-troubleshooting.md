# 🌐 Network Setup & Troubleshooting Guide

This guide covers network configuration, firewall settings, and solutions to common issues when running **OBS HomeRun**.

---

## 📋 Table of Contents
1. [Network Requirements & Multicast (SSDP)](#1-network-requirements--multicast-ssdp)
2. [Firewall Configuration](#2-firewall-configuration)
   - [Fedora / Red Hat / Bazzite (`firewalld`)](#fedora--red-hat--bazzite-firewalld)
   - [Ubuntu / Debian (`ufw`)](#ubuntu--debian-ufw)
   - [Windows Defender Firewall (PowerShell)](#windows-defender-firewall-powershell)
   - [macOS Firewall & Local Network Privacy](#macos-firewall--local-network-privacy)
3. [Wi-Fi & Router Settings](#3-wi-fi--router-settings)
4. [Troubleshooting Common Issues](#4-troubleshooting-common-issues)
   - [Issue 1: TV Does Not Discover OBS HomeRun](#issue-1-tv-does-not-discover-obs-homerun)
   - [Issue 2: Playback Stalls or Shows Infinite Loading Spinner](#issue-2-playback-stalls-or-shows-infinite-loading-spinner)
   - [Issue 3: Video is Smooth but No Audio](#issue-3-video-is-smooth-but-no-audio)
   - [Issue 4: Desktop Canvas is Zoomed or Cropped](#issue-4-desktop-canvas-is-zoomed-or-cropped)
   - [Issue 5: Flatpak OBS Cannot Connect to RTMP](#issue-5-flatpak-obs-cannot-connect-to-rtmp)

---

## 1. Network Requirements & Multicast (SSDP)

OBS HomeRun emulates a physical HDTV tuner over UPnP/DLNA using **SSDP (Simple Service Discovery Protocol)**.

* **Multicast Address:** `239.255.255.250`
* **Multicast Port:** `1900` (UDP)
* **HTTP Tuner & DLNA Port:** `5004` (TCP)
* **OBS Ingest Port:** `1935` (TCP - RTMP)

> ⚠️ **Critical: Host Networking Required:**  
> When running the container, you **must** use `--net=host` (or `network_mode: host` / Quadlet `Network=host`). Bridge networks isolate the container's UDP broadcast domain, preventing SSDP multicast packets from reaching your physical home network.

---

## 2. Firewall Configuration

If your PC runs a software firewall, ensure ports `1900/udp`, `5004/tcp`, and `1935/tcp` are allowed.

### Fedora / Red Hat / Bazzite (`firewalld`)

```bash
# Allow SSDP multicast discovery
sudo firewall-cmd --permanent --add-port=1900/udp

# Allow HTTP virtual tuner & DLNA streaming
sudo firewall-cmd --permanent --add-port=5004/tcp

# Allow OBS RTMP streaming ingest
sudo firewall-cmd --permanent --add-port=1935/tcp

# Reload firewall rules
sudo firewall-cmd --reload
```

### Ubuntu / Debian (`ufw`)

```bash
sudo ufw allow 1900/udp comment "OBS HomeRun SSDP"
sudo ufw allow 5004/tcp comment "OBS HomeRun DLNA"
sudo ufw allow 1935/tcp comment "OBS HomeRun RTMP"
sudo ufw reload
```

### Windows Defender Firewall (PowerShell)

Run PowerShell as Administrator:
```powershell
New-NetFirewallRule -DisplayName "OBS HomeRun SSDP" -Direction Inbound -Protocol UDP -LocalPort 1900 -Action Allow
New-NetFirewallRule -DisplayName "OBS HomeRun DLNA HTTP" -Direction Inbound -Protocol TCP -LocalPort 5004 -Action Allow
New-NetFirewallRule -DisplayName "OBS HomeRun RTMP Ingest" -Direction Inbound -Protocol TCP -LocalPort 1935 -Action Allow
```

### macOS Firewall & Local Network Privacy

1. **macOS Application Firewall:** Open **System Settings** $\rightarrow$ **Network** $\rightarrow$ **Firewall** $\rightarrow$ **Options...** and ensure `obs-homerun` and `mediamtx` are permitted to accept incoming connections.
2. **Local Network Permission (macOS 15 Sequoia / Sonoma):** When prompted with *“obs-homerun would like to find devices on local networks”*, select **Allow**.

> 💡 **Detailed Container Hosting Instructions:** For step-by-step guides using Docker, Podman, and NAS platforms across all OSs, see the [🐳 Container Hosting Guide](host-os-setup.md).

---

## 3. Wi-Fi & Router Settings

Smart TV discovery issues are almost always caused by home router multicast suppression. Verify these settings in your router's admin console:

1. **AP / Client Isolation (Must be DISABLED):**
   - Setting names: *AP Isolation*, *Client Isolation*, or *Guest Network Isolation*.
   - If enabled, wireless devices cannot see or communicate with other devices on your LAN.
2. **IGMP Snooping (Must be ENABLED):**
   - Allows switches and Wi-Fi access points to route multicast packets directly to subscribing devices.
3. **Multicast Enhancement / IGMP Proxy (ENABLE if available):**
   - On Asus, Netgear, or Ubiquiti routers, enable *Multicast Enhancement* or *Enable Wireless Multicast Forwarding* to ensure Wi-Fi connected TVs receive SSDP announcements.
4. **Subnet Matching:**
   - Your PC and Smart TV must be on the same local subnet (e.g. `192.168.1.xxx / 24`). If your PC is on Ethernet and your TV is on Wi-Fi, ensure your router does not isolate wired and wireless traffic into separate VLANs.

> 💡 **Wired Ethernet Advantage (vs. Wireless Casting):** Unlike wireless casting protocols (such as Miracast or AirPlay) that require peer-to-peer wireless proximity and suffer from RF interference, OBS HomeRun routes cleanly across your local network infrastructure. Streaming across wired Ethernet switches delivers a rock-solid, zero-packet-loss broadcast with crisp text and zero frame drops.

---

## 4. Troubleshooting Common Issues

### Issue 1: TV Does Not Discover OBS HomeRun
* **Symptom:** OBS HomeRun does not appear in the TV's Inputs, Home Dashboard, or Media Server list.
* **Checks:**
  1. **Verify container is running with host networking:**
     ```bash
     podman ps  # or docker ps
     ```
     Ensure port 5004 is accessible from another device on your network:
     ```bash
     curl http://<YOUR-PC-IP>:5004/discover.json
     ```
  2. **Verify SSDP listener is active:**
     ```bash
     ss -u -a | grep 1900
     ```
  3. **Check router AP Isolation:** Ensure Wi-Fi client isolation is disabled.
  4. **Force TV Network Rediscovery:** Turn off the TV, unplug power for 10 seconds (power cycle to clear TV UPnP cache), and turn it back on.

---

### Issue 2: Playback Stalls or Shows Infinite Loading Spinner
* **Symptom:** The stream connects for 2–5 seconds, then pauses, buffers, or spins the loading wheel.
* **Checks:**
  1. **Keyframe Interval in OBS:**  
     Ensure OBS **Keyframe Interval** is explicitly set to **`1 s`** (not `0` or auto). The TV's hardware decoder needs frequent IDR recovery frames to maintain sync.
  2. **B-frames in OBS:**  
     Set **Max B-frames** to **`0`**. B-frames require decoder reordering buffers that can stall live broadcast streams on some TV chipsets.
  3. **Increase Jitter Buffer:**  
     If on 2.4GHz Wi-Fi with micro-packet loss, increase the buffer safety duration:
     ```ini
     Environment=BUFFER_SECONDS=4.0
     ```
     (Default is `3.0` seconds).

---

### Issue 3: Video is Smooth but No Audio
* **Symptom:** Video plays smoothly at 60fps, but the TV is silent.
* **Checks:**
  1. **Audio Codec Setting:**  
     By default, `obs-homerun` transcodes audio to **AC-3 (Dolby Digital)**, the native standard for broadcast HDTV. Ensure `AUDIO_CODEC=ac3` is set (or left at default).
  2. **OBS Audio Sample Rate:**  
     Set OBS **Settings** $\rightarrow$ **Audio** $\rightarrow$ **Sample Rate** to **`48 kHz`**. Broadcast decoders often mute or reject 44.1 kHz feeds.
  3. **Audio Device Active in OBS:**  
     Ensure the OBS audio mixer shows green/yellow level activity on the **Desktop Audio** track.

---

### Issue 4: Desktop Canvas is Zoomed or Cropped
* **Symptom:** Desktop edges or taskbar are cut off on the TV screen.
* **Checks:**
  1. **OBS Transform Reset:**  
     In OBS, click your screen capture source in the preview window and press **`Ctrl + R`** (Reset Transform), followed by **`Ctrl + F`** (Fit to Screen).
  2. **TV Aspect Ratio Setting:**  
     On your TV, open Picture / Display settings and set **Aspect Ratio** to **`Original`**, **`Just Scan`**, **`Fit to Screen`**, or **`1:1 Pixel Mapping`** (depending on TV brand). Disable *16:9 Overscan* or *Zoom*.

---

### Issue 5: Flatpak OBS Cannot Connect to RTMP
* **Symptom:** OBS fails to connect with *"Failed to connect to server"* when streaming to `rtmp://localhost:1935/live`.
* **Fix:**  
  Flatpak sandboxes isolate `localhost`. Change the OBS Stream Server to your PC's LAN IP:
  ```
  rtmp://192.168.1.xxx:1935/live
  ```
  Or grant OBS network access via Flatseal.
