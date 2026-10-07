use axum::{routing::get, Router};
use obs_homerun::{config::Config, handlers, ssdp, stream};
use std::sync::Arc;
use tracing::{info, warn};

fn find_file_candidates(
    env_var: &str,
    relative_names: &[&str],
    system_paths: &[&str],
) -> Option<std::path::PathBuf> {
    if let Ok(val) = std::env::var(env_var) {
        let p = std::path::PathBuf::from(val);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in relative_names {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    for name in relative_names {
        let p = std::path::PathBuf::from(name);
        if p.is_file() {
            return Some(p);
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in relative_names {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    for p_str in system_paths {
        let p = std::path::PathBuf::from(p_str);
        if p.is_file() {
            return Some(p);
        }
    }

    None
}

fn discover_mediamtx() -> (Option<std::path::PathBuf>, Option<std::path::PathBuf>) {
    let exe_names: Vec<&str> = if cfg!(windows) {
        vec!["mediamtx.exe", "mediamtx"]
    } else {
        vec!["mediamtx", "mediamtx.exe"]
    };

    let bin_path = find_file_candidates(
        "MEDIAMTX_BIN",
        &exe_names,
        &[
            "/app/mediamtx",
            "/opt/homebrew/bin/mediamtx",
            "/usr/local/bin/mediamtx",
            "/usr/bin/mediamtx",
        ],
    );

    let mut config_path = find_file_candidates(
        "MEDIAMTX_CONFIG",
        &["mediamtx.yml"],
        &[
            "/app/mediamtx.yml",
            "/etc/mediamtx.yml",
            "/usr/local/etc/mediamtx.yml",
        ],
    );

    // If no config found in standard paths, check co-located with binary
    if config_path.is_none() {
        if let Some(ref bin) = bin_path {
            if let Some(parent) = bin.parent() {
                let candidate = parent.join("mediamtx.yml");
                if candidate.is_file() {
                    config_path = Some(candidate);
                }
            }
        }
    }

    (bin_path, config_path)
}

fn spawn_mediamtx_supervisor(
    bin: std::path::PathBuf,
    config: Option<std::path::PathBuf>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while !*shutdown_rx.borrow() {
            let mut cmd = tokio::process::Command::new(&bin);
            if let Some(ref cfg) = config {
                info!(
                    "[MediaMTX] Starting engine ({}) with config ({})...",
                    bin.display(),
                    cfg.display()
                );
                cmd.arg(cfg);
            } else {
                info!(
                    "[MediaMTX] Starting engine ({}) with default configuration...",
                    bin.display()
                );
            }

            cmd.stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .kill_on_drop(true);

            match cmd.spawn() {
                Ok(mut child) => {
                    tokio::select! {
                        res = child.wait() => {
                            if *shutdown_rx.borrow() {
                                break;
                            }
                            match res {
                                Ok(status) => {
                                    warn!(
                                        "[MediaMTX] Process terminated unexpectedly with status: {status}. Restarting in 1s..."
                                    );
                                }
                                Err(e) => {
                                    warn!(
                                        "[MediaMTX] Error waiting on process: {e}. Restarting in 1s..."
                                    );
                                }
                            }
                        }
                        _ = shutdown_rx.changed() => {
                            if *shutdown_rx.borrow() {
                                let _ = child.kill().await;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                    warn!("[MediaMTX] Failed to spawn MediaMTX: {e}. Retrying in 2s...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
            }

            // Brief backoff before restart unless shutting down
            tokio::select! {
                () = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
            }
        }
        info!("[MediaMTX] Engine stopped.");
    })
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

    let friendly_name = &config.friendly_name;
    let channel_number = &config.channel_number;
    let host_ip = &config.host_ip;
    let http_port = config.http_port;
    let rtsp_source = &config.rtsp_source;
    let buffer_sec = config.buffer_sec;
    let half_buffer = &config.half_buffer;
    let audio_codec = &config.audio_codec;
    let audio_bitrate = &config.audio_bitrate;

    info!("==================================================");
    info!("       Starting OBS HomeRun (Rust Engine)         ");
    info!("==================================================");
    info!("Friendly Name:   {friendly_name}");
    info!("Channel Number:  {channel_number}");
    info!("Host Address:    {host_ip}:{http_port}");
    info!("RTSP Source:     {rtsp_source}");
    info!("Buffer Safety:   {buffer_sec:.1}s (muxdelay: {half_buffer}s)");
    info!("Audio Codec:     {audio_codec} ({audio_bitrate})");
    info!("==================================================");

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let (bin_path, config_path) = discover_mediamtx();
    let _mtx_handle = bin_path.map_or_else(
        || {
            info!("[Init] No local MediaMTX binary found. Expecting external MediaMTX service.");
            None
        },
        |bin| Some(spawn_mediamtx_supervisor(bin, config_path, shutdown_rx)),
    );

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
    info!("HTTP server listening on http://0.0.0.0:{http_port}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    let _ = shutdown_tx.send(true);
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
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("Shutdown signal received, shutting down gracefully...");
}
