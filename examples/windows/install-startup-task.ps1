# OBS HomeRun - Install as Silent Background Task on User Logon
# Run this script in PowerShell as Administrator

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "       OBS HomeRun - Windows Background Task Installer          " -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exePath = Join-Path $scriptDir "obs-homerun.exe"

if (-not (Test-Path $exePath)) {
    Write-Warning "obs-homerun.exe not found in $scriptDir! Please place this script in the same folder as obs-homerun.exe."
    Pause
    exit
}

$taskName = "OBSHomeRun"

Write-Host "Registering scheduled task '$taskName'..."
$Action = New-ScheduledTaskAction -Execute $exePath -WorkingDirectory $scriptDir
$Trigger = New-ScheduledTaskTrigger -AtLogOn
$Principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0

Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
Register-ScheduledTask -TaskName $taskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings | Out-Null
Start-ScheduledTask -TaskName $taskName

Write-Host "`nTask '$taskName' installed and started successfully!" -ForegroundColor Green
Write-Host "OBS HomeRun will now run automatically in the background whenever you log into Windows." -ForegroundColor Green
Write-Host "================================================================" -ForegroundColor Cyan
