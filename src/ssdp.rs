use crate::config::Config;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, warn};

pub async fn run_ssdp(config: Arc<Config>) {
    let mcast_group = Ipv4Addr::new(239, 255, 255, 250);
    let mcast_port = 1900;
    let mcast_addr: SocketAddr = SocketAddrV4::new(mcast_group, mcast_port).into();

    let targets = vec![
        "upnp:rootdevice".to_string(),
        format!("uuid:{}", config.device_uuid),
        "urn:schemas-upnp-org:device:MediaServer:1".to_string(),
        "urn:schemas-upnp-org:service:ContentDirectory:1".to_string(),
        "urn:schemas-upnp-org:service:ConnectionManager:1".to_string(),
    ];

    // Spawn periodic NOTIFY task
    let config_notify = Arc::clone(&config);
    let targets_notify = targets.clone();
    tokio::spawn(async move {
        let sender_socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                error!("[SSDP] Failed to bind NOTIFY sender UDP socket: {}", e);
                return;
            }
        };

        let send_notify = |socket: &UdpSocket| {
            for t in &targets_notify {
                let usn = if t == &format!("uuid:{}", config_notify.device_uuid) {
                    format!("uuid:{}", config_notify.device_uuid)
                } else {
                    format!("uuid:{}::{}", config_notify.device_uuid, t)
                };

                let msg = format!(
                    "NOTIFY * HTTP/1.1\r\n\
                     HOST: 239.255.255.250:1900\r\n\
                     NT: {target}\r\n\
                     NTS: ssdp:alive\r\n\
                     LOCATION: http://{host}:{port}/dms/device.xml\r\n\
                     USN: {usn}\r\n\
                     CACHE-CONTROL: max-age=1800\r\n\
                     SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n\r\n",
                    target = t,
                    host = config_notify.host_ip,
                    port = config_notify.http_port,
                    usn = usn
                );
                let _ = socket.try_send_to(msg.as_bytes(), mcast_addr);
            }
        };

        // Initial broadcast
        send_notify(&sender_socket);

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            send_notify(&sender_socket);
        }
    });

    // Setup multicast listener for M-SEARCH queries
    let listener = match (|| -> std::io::Result<UdpSocket> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        #[cfg(not(windows))]
        socket.set_reuse_port(true)?;
        socket.set_nonblocking(true)?;

        let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, mcast_port);
        socket.bind(&socket2::SockAddr::from(bind_addr))?;
        socket.join_multicast_v4(&mcast_group, &Ipv4Addr::UNSPECIFIED)?;

        let std_socket: std::net::UdpSocket = socket.into();
        UdpSocket::from_std(std_socket)
    })() {
        Ok(s) => s,
        Err(e) => {
            warn!("[SSDP] Multicast listen error on 239.255.255.250:1900: {} (M-SEARCH response disabled)", e);
            return;
        }
    };

    info!("[SSDP] Listening on 239.255.255.250:1900...");

    let mut buf = vec![0u8; 2048];
    loop {
        let Ok((n, client_addr)) = listener.recv_from(&mut buf).await else {
            break;
        };

        let Ok(text) = std::str::from_utf8(&buf[..n]) else {
            continue;
        };

        if !text.contains("M-SEARCH") {
            continue;
        }

        let mut st = None;
        for line in text.lines() {
            let upper = line.to_uppercase();
            if upper.starts_with("ST:") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    st = Some(parts[1].trim());
                }
                break;
            }
        }

        let st = match st {
            Some(s) if !s.is_empty() => s,
            _ => continue,
        };

        let matched: Vec<String> = if st == "ssdp:all" {
            targets.clone()
        } else {
            targets.iter().filter(|t| *t == st).cloned().collect()
        };

        for m in matched {
            let usn = if m == format!("uuid:{}", config.device_uuid) {
                format!("uuid:{}", config.device_uuid)
            } else {
                format!("uuid:{}::{m}", config.device_uuid)
            };

            let host = &config.host_ip;
            let port = config.http_port;
            let resp = format!(
                "HTTP/1.1 200 OK\r\n\
                 CACHE-CONTROL: max-age=1800\r\n\
                 EXT:\r\n\
                 LOCATION: http://{host}:{port}/dms/device.xml\r\n\
                 SERVER: Linux/UPnP/1.0 DLNADOC/1.50 VirtualHDTV/1.0\r\n\
                 ST: {m}\r\n\
                 USN: {usn}\r\n\r\n"
            );
            let _ = listener.send_to(resp.as_bytes(), client_addr).await;
        }
    }
}
