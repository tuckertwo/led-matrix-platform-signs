use crate::config;
use crate::config::{ConfigStore, PW_STORE_ID, SSID_STORE_ID};
use crate::net_utils;
use crate::net_utils::net_task;
use core::convert::identity;
use core::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use defmt::{error, info, warn};
use edge_dhcp::server::{Server, ServerOptions};
use edge_nal::UdpBind;
use edge_nal_embassy::{Udp, UdpBuffers};
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_storage::FlashStorageError;
use esp_wifi::wifi::{
    AccessPointConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState,
};
use percent_encoding::percent_decode_str;
use smoltcp::wire::Ipv4Cidr;
use static_cell::make_static;

const GATEWAY_ADDR: Ipv4Addr = Ipv4Addr::new(192, 168, 2, 1);

pub async fn spawn_captive_portal(
    spawner: &Spawner,
    rng_seed: u64,
    dev: WifiDevice<'static>,
    mut controller: WifiController<'static>,
) {
    let net_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(GATEWAY_ADDR, 24),
        gateway: Some(GATEWAY_ADDR),
        dns_servers: heapless::Vec::from_slice(&[GATEWAY_ADDR]).unwrap(),
    });

    info!("Spawning captive portal");

    controller.disconnect_async().await.unwrap();
    controller.stop_async().await.unwrap();

    spawner.spawn(connection_task(controller)).ok();

    let (ap_stack, ap_runner) = embassy_net::new(
        dev,
        net_config,
        make_static!(StackResources::<5>::new()),
        rng_seed,
    );

    spawner.spawn(net_task(ap_runner)).ok();
    net_utils::wait_for_network_ready(ap_stack).await;

    spawner.spawn(dhcp_task(ap_stack, GATEWAY_ADDR)).ok();
    spawner.spawn(captive_dns_task(ap_stack)).ok();
    spawner.spawn(ap_task(ap_stack)).ok();
}

#[embassy_executor::task]
async fn ap_task(net_stack: Stack<'static>) {
    info!("Starting AP task");
    let mut rx_buffer = [0; 1536];
    let mut tx_buffer = [0; 1536];

    let mut socket = TcpSocket::new(net_stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    loop {
        info!("Waiting for connection...");

        let r = socket
            .accept(smoltcp::wire::IpListenEndpoint {
                addr: None,
                port: 80,
            })
            .await;

        if let Err(e) = r {
            warn!("Failed to connect: {:?}", e);
            continue;
        }
        info!("Connected");

        let mut buffer = [0u8; 1024];
        let mut pos = 0;
        let request: Option<&str> = loop {
            match socket.read(&mut buffer).await {
                Ok(0) => {
                    info!("read EOF");
                    break None;
                }
                Ok(len) => {
                    let request = unsafe { core::str::from_utf8_unchecked(&buffer[..(pos + len)]) };
                    if request.contains("\r\n\r\n") {
                        info!("{}", request);
                        break Some(request);
                    } else {
                        pos += len;
                    }
                }
                Err(e) => {
                    error!("read error: {:?}", e);
                    break None;
                }
            }
        };

        if let Some(request) = request
            && request.starts_with("POST / ")
        {
            let r = socket.write_all(b"HTTP/1.0 200 OK\r\n\r\n").await;
            if let Err(e) = r {
                warn!("Failed to write response: {:?}", e);
            }
            let r = socket.flush().await;
            if let Err(e) = r {
                warn!("Failed to flush socket: {:?}", e);
            }

            if let Some((ssid, pw)) = try {
                let mut parts = (&request[request.find("\r\n\r\n")? + 4..])
                    .split('&')
                    .map(|v| {
                        let mut s = v.split('=');
                        Some((s.next()?, s.next()?))
                    });
                let mut fk = |k| parts.find_map(|v| if v?.0 == k { Some(v?.1) } else { None });
                (fk("ssid")?, fk("pw")?)
            } {
                let decode = |s: &str| {
                    let mut decoded = heapless::Vec::<u8, { config::CONFIG_ENTRY_LEN }>::new();
                    percent_decode_str(s).collect_into(&mut decoded);
                    heapless::String::<{ config::CONFIG_ENTRY_LEN }>::from_utf8(decoded).unwrap()
                };
                let ssid = decode(ssid);
                let ssid = ssid.as_str();
                let pw = decode(pw);
                let pw = pw.as_str();
                info!("SSID: {}, PW: {}", ssid, pw);
                let mut c = ConfigStore::new();
                let _ = c.set(SSID_STORE_ID, ssid).inspect_err(|e| match e {
                    FlashStorageError::Other(i) => error!("flash storage error {}", i),
                    _ => error!("other flash error"),
                });
                let _ = c.set(PW_STORE_ID, pw).inspect_err(|e| match e {
                    FlashStorageError::Other(i) => error!("flash storage error {}", i),
                    _ => error!("other flash error"),
                });
                info!("wrote to flash, resetting system to try to connect");
                socket.close();
                socket.abort();
                esp_hal::system::software_reset();
            } else {
                info!("Failed to parse request");
            }
        } else {
            if let Some(request) = request {
                info!("handling request");
                let r = socket
                    .write_all(if request.starts_with("GET / ") {
                        info!("sending index.html");
                        include_bytes!("./web/index.html")
                    } else {
                        info!("sending 302");
                        b"HTTP/1.1 302 Found\r\nLocation: http://192.168.2.1\r\n\r\n"
                    })
                    .await;
                if let Err(e) = r {
                    warn!("Failed to write response: {:?}", e);
                }
            }
            let r = socket.flush().await;
            if let Err(e) = r {
                warn!("Failed to flush socket: {:?}", e);
            }
        }

        // Timer::after_millis(1000).await;
        socket.close();
        // Timer::after_millis(1000).await;
        socket.abort();
    }
}

#[embassy_executor::task]
async fn dhcp_task(stack: Stack<'static>, gw_ip_addr: Ipv4Addr) {
    info!("Starting DHCP server...");

    let mut buf = [0u8; 512];
    let mut gw_buf = [Ipv4Addr::UNSPECIFIED];

    let buffers = UdpBuffers::<1, 512, 512, 1>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            edge_dhcp::io::DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    let mut dhcp_options = ServerOptions::new(gw_ip_addr, Some(&mut gw_buf));
    dhcp_options.captive_url = Some("http://192.168.2.1");
    dhcp_options.dns = &[GATEWAY_ADDR];

    loop {
        _ = edge_dhcp::io::server::run(
            &mut Server::<_, 64>::new_with_et(gw_ip_addr),
            &dhcp_options,
            &mut bound_socket,
            &mut buf,
        )
        .await
        .inspect_err(|_e| warn!("DHCP server error"));
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    info!("Start connection task");

    loop {
        if esp_wifi::wifi::ap_state() == WifiState::ApStarted {
            info!("Waiting until no longer connected");
            controller.wait_for_event(WifiEvent::ApStop).await;
            Timer::after_millis(5000).await;
        }

        if !controller.is_started().is_ok_and(identity) {
            let client_config = Configuration::AccessPoint(AccessPointConfiguration {
                ssid: "led-matrix".into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started");
        }
    }
}

#[embassy_executor::task]
async fn captive_dns_task(net_stack: Stack<'static>) {
    info!("Start DNS task");

    let buffers = UdpBuffers::<1, 512, 512, 1>::new();
    let udp = Udp::new(net_stack, &buffers);

    let mut tx_buf = [0u8; 512];
    let mut rx_buf = [0u8; 1024];

    edge_captive::io::run(
        &udp,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 53),
        &mut tx_buf,
        &mut rx_buf,
        GATEWAY_ADDR,
        core::time::Duration::from_secs(60),
    )
    .await
    .unwrap();
}
