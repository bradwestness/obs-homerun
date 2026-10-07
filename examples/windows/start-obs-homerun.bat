@echo off
setlocal
title OBS HomeRun

echo ================================================================
echo                    Starting OBS HomeRun...
echo ================================================================
echo  * Virtual HDTV Tuner is starting on port 5004
echo  * RTMP ingest is listening on rtmp://localhost:1935/live
echo  * Smart TVs on your Wi-Fi/Ethernet will discover this tuner!
echo.
echo  Keep this window open while streaming to your Smart TV.
echo  Press Ctrl+C to stop.
echo ================================================================
echo.

"%~dp0obs-homerun.exe"

if errorlevel 1 (
    echo.
    echo OBS HomeRun stopped unexpectedly.
    pause
)
