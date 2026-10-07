use axum::{routing::get, Router};
use obs_homerun::{config::Config, handlers, ssdp, stream};
use std::sync::Arc;
use tracing::{info, warn};

fn start_mediamtx() -> Option<tokio::process::Child> {
    let bin_path = std::env::var("MEDIAMTX_BIN").unwrap_or_else(|_| "/app/mediamtx".to_string());
    let config_path = std::env::var("MEDIAMTX_CONFIG").unwrap_or_else(|_| "/app/mediamtx.yml".to_string());

    if std::path::Path::new(&bin_path).exists() {
        info!("[Init] Starting embedded MediaMTX engine ({})...", bin_path);
        let mut cmd = tokio::process::Command::new(&bin_path);
        cmd.arg(&config_path)
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .kill_on_drop(true);

        match cmd.spawn() {
            Ok(child) => return Some(child),
            Err(e) => {
                warn!("[Init] Warning: Failed to start MediaMTX: {}", e);
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Arc::new(Config::from_env());

    info!("==================================================");
    info!("       Starting OBS HomeRun (Rust Engine)         ");
    info!("==================================================");
    info!("Friendly Name:   {}", config.friendly_name);
    info!("Channel Number:  {}", config.channel_number);
    info!("Host Address:    {}:{}", config.host_ip, config.http_port);
    info!("RTSP Source:     {}", config.rtsp_source);
    info!("Buffer Safety:   {:.1}s (muxdelay: {}s)", config.buffer_sec, config.half_buffer);
    info!("Audio Codec:     {} ({})", config.audio_codec, config.audio_bitrate);
    info!("==================================================");

    let _mtx_child = start_mediamtx();

    // Start SSDP background task
    tokio::spawn(ssdp::run_ssdp(Arc::clone(&config)));

    let app = Router::new()
        .route("/discover.json", get(handlers::handle_discover))
        .route("/lineup.json", get(handlers::handle_lineup))
        .route("/dms/device.xml", get(handlers::handle_device_xml))
        .route(
            "/dms/ConnectionManager.xml",
            get(handlers::handle_connection_manager).post(handlers::handle_connection_manager),
        )
        .route(
            "/dms/ConnectionManager",
            get(handlers::handle_connection_manager).post(handlers::handle_connection_manager),
        )
        .route(
            "/dms/ContentDirectory.xml",
            get(handlers::handle_content_directory).post(handlers::handle_content_directory),
        )
        .route(
            "/dms/ContentDirectory",
            get(handlers::handle_content_directory).post(handlers::handle_content_directory),
        )
        .route(
            "/auto/*path",
            get(stream::handle_stream).head(stream::handle_stream),
        )
        .route(
            "/live",
            get(stream::handle_stream).head(stream::handle_stream),
        )
        .route(
            "/stream.ts",
            get(stream::handle_stream).head(stream::handle_stream),
        )
        .with_state(Arc::clone(&config));

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], config.http_port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("HTTP server listening on http://0.0.0.0:{}", config.http_port);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("OBS HomeRun exited.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, shutting down gracefully...");
}
