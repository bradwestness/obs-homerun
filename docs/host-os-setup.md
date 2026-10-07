# Container Setup Guide (Podman & Docker)

OBS HomeRun is distributed as a single container image (`ghcr.io/bradwestness/obs-homerun:latest`) packaging the Rust broadcaster engine, MediaMTX for RTMP ingest, and FFmpeg for on-demand MPEG-TS remuxing.

Because Smart TVs discover the stream using UPnP/SSDP multicast (`239.255.255.250:1900` UDP), the container must run with host networking (`--net=host` or `network_mode: host`). Bridge networks isolate multicast packets inside the container engine and prevent TVs from finding the virtual tuner.

---

## Table of Contents
1. [Preferred Method: Podman Quadlet (Linux / Bazzite / SteamOS / Fedora)](#preferred-method-podman-quadlet-linux--bazzite--steamos--fedora)
2. [Linux: Docker Compose / Docker CLI](#linux-docker-compose--docker-cli)
3. [Windows 11: Docker Desktop](#windows-11-docker-desktop)
4. [macOS: Docker Desktop](#macos-docker-desktop)
5. [Dedicated Home Server / NAS (Synology, Unraid, TrueNAS)](#dedicated-home-server--nas-synology-unraid-truenas)
6. [Verifying the Setup](#verifying-the-setup)
7. [Next Steps: OBS Studio Configuration](#next-steps-obs-studio-configuration)

---

## Preferred Method: Podman Quadlet (Linux / Bazzite / SteamOS / Fedora)

If you are on Linux—especially atomic/immutable gaming distros like **Bazzite**, **SteamOS** (Steam Deck desktop mode), or **Fedora Silverblue**—Podman Quadlets are the preferred deployment method.

Quadlets let systemd manage the container directly as a rootless user service. With `AutoUpdate=registry`, systemd can automatically check ghcr.io for new image builds and update the container in the background without manual intervention.

### 1. Create the Quadlet file

Create `~/.config/containers/systemd/obs-homerun.container`:

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

### 2. Start and enable the service

```bash
# Reload systemd generator to pick up the new Quadlet
systemctl --user daemon-reload

# Start the container service
systemctl --user start obs-homerun.service

# Keep user services running after logout/reboot
loginctl enable-linger $USER
```

### 3. Automatic Updates

Because `AutoUpdate=registry` is defined in the Quadlet, you can trigger auto-updates manually or enable the built-in systemd timer:

```bash
# Check for and apply newer images from the registry
podman auto-update

# Or enable the systemd auto-update timer to run daily
systemctl --user enable --now podman-auto-update.timer
```

Whenever a new image is pushed to `latest`, Podman pulls it and restarts the service automatically.

### 4. Firewall rules

Ensure incoming discovery and stream traffic are allowed:

**Fedora / Bazzite / RHEL (`firewalld`):**
```bash
sudo firewall-cmd --permanent --add-port={1900/udp,5004/tcp,1935/tcp}
sudo firewall-cmd --reload
```

**Ubuntu / Debian (`ufw`):**
```bash
sudo ufw allow 1900/udp
sudo ufw allow 5004/tcp
sudo ufw allow 1935/tcp
sudo ufw reload
```

---

## Linux: Docker Compose / Docker CLI

If you run Docker instead of Podman:

### Docker Compose

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

Start the container:
```bash
docker compose up -d
```

### Docker CLI

```bash
docker run -d \
  --name obs-homerun \
  --net=host \
  --restart=unless-stopped \
  ghcr.io/bradwestness/obs-homerun:latest
```

---

## Windows 11: Docker Desktop

For Windows, Docker Desktop with WSL2 is the standard option.

By default, WSL2 uses NAT, which drops UDP multicast packets before they reach your local Wi-Fi or Ethernet network. Windows 11 (build 22H2+) includes **WSL2 Mirrored Networking**, which shares the host network adapter directly with WSL2 and enables SSDP multicast discovery.

### 1. Enable WSL2 Mirrored Networking

Open `%USERPROFILE%\.wslconfig` (create it if it doesn't exist) and add:

```ini
[wsl2]
networkingMode=mirrored
firewall=true
```

*(An example file is provided at `examples/windows/.wslconfig`).*

Restart WSL in PowerShell:
```powershell
wsl --shutdown
```

Restart Docker Desktop after WSL has stopped.

### 2. Run the Container

Open PowerShell:
```powershell
docker run -d --name obs-homerun --net=host --restart=unless-stopped ghcr.io/bradwestness/obs-homerun:latest
```

### 3. Open Windows Defender Firewall

Run PowerShell as Administrator to allow incoming connections on the required ports:

```powershell
New-NetFirewallRule -DisplayName "OBS HomeRun SSDP" -Direction Inbound -Protocol UDP -LocalPort 1900 -Action Allow
New-NetFirewallRule -DisplayName "OBS HomeRun DLNA HTTP" -Direction Inbound -Protocol TCP -LocalPort 5004 -Action Allow
New-NetFirewallRule -DisplayName "OBS HomeRun RTMP Ingest" -Direction Inbound -Protocol TCP -LocalPort 1935 -Action Allow
```

*(You can also run `examples/windows/setup-firewall.ps1` as Administrator).*

---

## macOS: Docker Desktop

For macOS, Docker Desktop 4.34+ supports host networking:

```bash
docker run -d \
  --name obs-homerun \
  --net=host \
  --restart=unless-stopped \
  ghcr.io/bradwestness/obs-homerun:latest
```

When prompted:
- Allow Docker to accept incoming connections in the macOS Application Firewall.
- On macOS 15 (Sequoia) or newer, allow Docker to access devices on your local network.

*Note: If you run an always-on NAS or Linux box on your home network, hosting the container there instead is usually simpler than running Docker Desktop continuously on a laptop.*

---

## Dedicated Home Server / NAS (Synology, Unraid, TrueNAS)

Running the container on an always-on NAS or home server lets your Smart TV discover the tuner 24/7 without needing any containers or services running on your workstation or gaming machine.

### Synology DSM (Container Manager)

1. Open **Container Manager** in DSM.
2. Go to **Project** $\rightarrow$ **Create**.
3. Set project name to `obs-homerun`, select a folder path, and choose **Create docker-compose.yml**.
4. Paste:
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
5. Complete the wizard. Container Manager will pull the image and start the container with host networking.

### Unraid

1. In the Unraid WebGUI, go to the **Docker** tab and click **Add Container**.
2. Set Name to `obs-homerun`.
3. Set Repository to `ghcr.io/bradwestness/obs-homerun:latest`.
4. Set **Network Type** to **Host** (`--net=host`).
5. Click **Apply**.

### TrueNAS SCALE

1. In TrueNAS, go to **Apps** $\rightarrow$ **Discover Apps** $\rightarrow$ **Custom App**.
2. Set Image to `ghcr.io/bradwestness/obs-homerun`, tag to `latest`.
3. Under **Network Configuration**, check **Host Network**.
4. Save and deploy.

## Verifying the Setup

Check that the container is running and healthy:
```bash
podman ps   # or docker ps
```
The status should report `(healthy)`.

Test the HTTP discovery endpoint from another machine or phone on your network:
```bash
curl -s http://<HOST-IP>:5004/discover.json
```

Verify the SSDP multicast socket is listening on UDP port 1900:
```bash
# Linux
ss -u -a | grep 1900

# macOS
netstat -an -p udp | grep 1900

# Windows PowerShell
Get-NetUDPEndpoint -LocalPort 1900
```

---

## Next Steps: OBS Studio Configuration

Once the container is running and healthy:
1. Open the [OBS Studio Configuration Guide](obs-configuration.md) to configure your stream destination (`rtmp://localhost:1935/live` or `<NAS-IP>`), 1s keyframe interval, and GPU hardware encoder (NVENC, AMF, QuickSync, Apple Silicon).
2. Click **Start Streaming** in OBS.
3. Switch your Smart TV input to **OBS HomeRun** (Channel 1.1) to watch.

---

## Related Guides
- [OBS Studio Configuration Guide](obs-configuration.md): Hardware encoder settings, ultrawide canvas framing, and 5.1 surround sound.
- [Network Setup & Troubleshooting Guide](network-and-troubleshooting.md): Router multicast/IGMP settings and troubleshooting common playback issues.
