use crate::config::Config;
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Method, Response, StatusCode},
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, warn};

fn build_ffmpeg_cmd(config: &Config) -> tokio::process::Command {
    let mut audio_args = vec!["-c:a".to_string(), "copy".to_string()];
    if config.audio_codec != "copy" {
        audio_args = vec![
            "-c:a".to_string(),
            config.audio_codec.clone(),
            "-b:a".to_string(),
            config.audio_bitrate.clone(),
        ];
    }

    let mut ffmpeg_args = vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "warning".to_string(),
        "-rtsp_transport".to_string(),
        "tcp".to_string(),
        "-timeout".to_string(),
        "5000000".to_string(), // 5s socket I/O timeout in microseconds
        "-analyzeduration".to_string(),
        "500000".to_string(),
        "-probesize".to_string(),
        "1000000".to_string(),
        "-i".to_string(),
        config.rtsp_source.clone(),
        "-c:v".to_string(),
        "copy".to_string(),
        "-bsf:v".to_string(),
        "dump_extra".to_string(),
    ];
    ffmpeg_args.extend(audio_args);
    ffmpeg_args.extend(vec![
        "-muxdelay".to_string(),
        config.half_buffer.clone(),
        "-muxpreload".to_string(),
        config.half_buffer.clone(),
        "-pat_period".to_string(),
        "0.1".to_string(),
        "-mpegts_flags".to_string(),
        "+initial_discontinuity+resend_headers".to_string(),
        "-f".to_string(),
        "mpegts".to_string(),
        "pipe:1".to_string(),
    ]);

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.args(&ffmpeg_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .kill_on_drop(true);
    cmd
}

async fn spawn_ffmpeg(
    config: &Config,
) -> Result<(tokio::process::Child, tokio::process::ChildStdout, Vec<u8>), String> {
    let mut cmd = build_ffmpeg_cmd(config);
    let mut child = cmd.spawn().map_err(|e| format!("spawn error: {}", e))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture stdout".to_string())?;

    let mut initial_buf = vec![0u8; 65536];
    let n = stdout
        .read(&mut initial_buf)
        .await
        .map_err(|e| format!("read error: {}", e))?;

    if n == 0 {
        return Err("EOF on initial read".to_string());
    }

    initial_buf.truncate(n);
    Ok((child, stdout, initial_buf))
}

pub async fn handle_stream(
    State(config): State<Arc<Config>>,
    method: Method,
) -> Result<Response<Body>, StatusCode> {
    let builder = Response::builder()
        .header(
            header::SERVER,
            HeaderValue::from_static("Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0"),
        )
        .header(header::CONNECTION, HeaderValue::from_static("close"))
        .header(header::CONTENT_TYPE, HeaderValue::from_static("video/mpeg"))
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .header(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        )
        .header(
            "transferMode.dlna.org",
            HeaderValue::from_static("Streaming"),
        )
        .header(
            "contentFeatures.dlna.org",
            HeaderValue::from_static(
                "DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000",
            ),
        );

    if method == Method::HEAD {
        return Ok(builder.status(StatusCode::OK).body(Body::empty()).unwrap());
    }

    info!("[Stream] Client connected, establishing stream pipeline...");

    let max_initial_retries = 3;
    let mut initial_attempt = 0;
    let (first_child, first_stdout, initial_buf) = loop {
        initial_attempt += 1;
        match spawn_ffmpeg(&config).await {
            Ok(res) => break res,
            Err(e) => {
                if initial_attempt >= max_initial_retries {
                    error!(
                        "[Stream] Failed to connect to stream after {} attempts: {}",
                        max_initial_retries, e
                    );
                    return Err(StatusCode::BAD_GATEWAY);
                }
                warn!(
                    "[Stream] Connection attempt {}/{} failed ({}), retrying in 500ms...",
                    initial_attempt, max_initial_retries, e
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }
    };

    let (duplex_read, mut duplex_write) = tokio::io::duplex(131072);
    let stream_config = Arc::clone(&config);

    tokio::spawn(async move {
        // Send initial buffered chunk
        if let Err(e) = duplex_write.write_all(&initial_buf).await {
            debug!("[Stream] Client disconnected before initial buffer sent: {}", e);
            let mut dead = first_child;
            let _ = dead.kill().await;
            return;
        }

        let mut current_child = first_child;
        let mut current_stdout = first_stdout;

        loop {
            // Stream bytes from FFmpeg stdout into duplex_write until EOF or client disconnect
            let copy_res = tokio::io::copy(&mut current_stdout, &mut duplex_write).await;
            let _ = current_child.kill().await;

            match copy_res {
                Ok(_) => {
                    info!("[Stream] Broadcaster stream ended or connection dropped.");
                }
                Err(e) => {
                    // Client disconnected (e.g. BrokenPipe)
                    info!("[Stream] Client connection closed: {}", e);
                    break;
                }
            }

            // Attempt to reconnect if client is still listening
            info!("[Stream] Attempting to reconnect to broadcaster (up to 10s grace period)...");
            let mut reconnected = false;
            let reconnect_deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);

            while tokio::time::Instant::now() < reconnect_deadline {
                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

                match spawn_ffmpeg(&stream_config).await {
                    Ok((new_child, new_stdout, new_buf)) => {
                        // Test writing to client to verify it hasn't disconnected
                        if let Err(e) = duplex_write.write_all(&new_buf).await {
                            info!("[Stream] Client disconnected during reconnect: {}", e);
                            let mut dead = new_child;
                            let _ = dead.kill().await;
                            return;
                        }
                        info!("[Stream] Reconnected successfully to stream! Resuming playback.");
                        current_child = new_child;
                        current_stdout = new_stdout;
                        reconnected = true;
                        break;
                    }
                    Err(_) => {
                        // Broadcaster not ready yet, keep waiting within grace period
                    }
                }
            }

            if !reconnected {
                warn!("[Stream] Broadcaster did not reconnect within 10s. Closing stream.");
                break;
            }
        }
    });

    let body = Body::from_stream(ReaderStream::new(duplex_read));
    Ok(builder.status(StatusCode::OK).body(body).unwrap())
}
