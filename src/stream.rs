use crate::config::Config;
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderValue, Method, Response, StatusCode},
};
use bytes::Bytes;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio_util::io::ReaderStream;
use tracing::{error, info};

pub async fn handle_stream(
    State(config): State<Arc<Config>>,
    method: Method,
) -> Result<Response<Body>, StatusCode> {
    let builder = Response::builder()
        .header(header::SERVER, HeaderValue::from_static("Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0"))
        .header(header::CONNECTION, HeaderValue::from_static("close"))
        .header(header::CONTENT_TYPE, HeaderValue::from_static("video/mpeg"))
        .header(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"))
        .header("transferMode.dlna.org", HeaderValue::from_static("Streaming"))
        .header(
            "contentFeatures.dlna.org",
            HeaderValue::from_static("DLNA.ORG_PN=AVC_TS_HD_60_AC3_ISO;DLNA.ORG_OP=00;DLNA.ORG_FLAGS=01700000000000000000000000000000"),
        );

    if method == Method::HEAD {
        return Ok(builder.status(StatusCode::OK).body(Body::empty()).unwrap());
    }

    info!("[Stream] Client connected, spawning FFmpeg remuxer...");

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

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            error!("[Stream] Failed to spawn FFmpeg: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            error!("[Stream] Failed to capture FFmpeg stdout");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Buffer initial chunk before returning HTTP 200 OK
    let mut initial_buf = vec![0u8; 65536];
    let n = match stdout.read(&mut initial_buf).await {
        Ok(n) => n,
        Err(e) => {
            error!("[Stream] Error reading initial buffer from FFmpeg: {}", e);
            return Err(StatusCode::BAD_GATEWAY);
        }
    };

    if n == 0 {
        error!("[Stream] FFmpeg stream unavailable (EOF on initial read)");
        return Err(StatusCode::BAD_GATEWAY);
    }

    initial_buf.truncate(n);
    let first_chunk = futures_util::stream::once(async move { Ok::<Bytes, std::io::Error>(Bytes::from(initial_buf)) });
    let rest_stream = ReaderStream::new(stdout);
    let combined_stream = first_chunk.chain(rest_stream);

    // Keep child process alive while stream is active
    let body_stream = combined_stream.map(move |item| {
        let _keep_child_alive = &child;
        item
    });

    let body = Body::from_stream(body_stream);
    Ok(builder.status(StatusCode::OK).body(body).unwrap())
}
