# OBS HomeRun - Windows Defender Firewall Setup Script
# Run this script in PowerShell as Administrator

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "         OBS HomeRun - Windows Defender Firewall Setup          " -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

# Check for Administrator privileges
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Warning "Please right-click this script and select 'Run with PowerShell' as Administrator!"
    Pause
    exit
}

# 1. Allow SSDP Multicast (Port 1900 UDP)
Write-Host "Allowing SSDP UPnP Multicast (Port 1900 UDP)..." -NoNewline
New-NetFirewallRule -DisplayName "OBS HomeRun SSDP" -Direction Inbound -Protocol UDP -LocalPort 1900 -Action Allow -ErrorAction SilentlyContinue | Out-Null
Write-Host " [OK]" -ForegroundColor Green

# 2. Allow DLNA & Virtual Tuner HTTP Streaming (Port 5004 TCP)
Write-Host "Allowing Virtual Tuner HTTP Streaming (Port 5004 TCP)..." -NoNewline
New-NetFirewallRule -DisplayName "OBS HomeRun DLNA HTTP" -Direction Inbound -Protocol TCP -LocalPort 5004 -Action Allow -ErrorAction SilentlyContinue | Out-Null
Write-Host " [OK]" -ForegroundColor Green

# 3. Allow RTMP Stream Ingest from OBS Studio (Port 1935 TCP)
Write-Host "Allowing RTMP Ingest from OBS Studio (Port 1935 TCP)..." -NoNewline
New-NetFirewallRule -DisplayName "OBS HomeRun RTMP Ingest" -Direction Inbound -Protocol TCP -LocalPort 1935 -Action Allow -ErrorAction SilentlyContinue | Out-Null
Write-Host " [OK]" -ForegroundColor Green

Write-Host "`nFirewall configuration complete! Smart TVs can now discover OBS HomeRun." -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Cyan
