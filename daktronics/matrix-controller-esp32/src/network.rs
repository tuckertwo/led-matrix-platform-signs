use crate::captive::spawn_captive_portal;
use crate::config::{ConfigStore, PW_STORE_ID, SSID_STORE_ID};
use crate::net_utils::{net_task, wait_for_network_ready};
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_time::Timer;
use esp_hal::peripherals::{RADIO_CLK, TIMG0, WIFI};
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_storage::FlashStorageError;
use esp_wifi::wifi::{ClientConfiguration, Configuration, WifiController, WifiEvent, WifiState};
use esp_wifi::{wifi, EspWifiController};
use static_cell::make_static;
// https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_access_point.rs

pub async fn net_init(
    spawner: &Spawner,
    timg0: TimerGroup<'static, TIMG0<'static>>,
    rng: &mut Rng,
    radio_clk: RADIO_CLK<'static>,
    wifi: WIFI<'static>,
) {
    let esp_wifi_ctrl: &'static mut EspWifiController =
        make_static!(esp_wifi::init(timg0.timer0, rng.clone(), radio_clk).unwrap());
    let (mut controller, interfaces) = wifi::new(esp_wifi_ctrl, wifi).unwrap();

    let rng_seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let mut config_store = ConfigStore::new();
    let ssid: Result<_, FlashStorageError> = try {
        (
            config_store.get(SSID_STORE_ID)?,
            config_store.get(PW_STORE_ID)?,
        )
    };
    if ssid.is_err() {
        info!("Failed to get SSID/pw from config store");
        spawn_captive_portal(spawner, rng_seed, interfaces.ap, controller).await;
        return;
    }

    let (ssid, pw) = ssid.unwrap();
    // try connecting
    info!("Connecting to {}:{}", ssid, pw);

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: ssid.as_str().into(),
        password: pw.as_str().into(),
        ..Default::default()
    });
    controller
        .set_configuration(&client_config)
        .inspect_err(|e| {
            error!("Failed to set wifi controller config: {:?}", e);
        })
        .unwrap();

    info!("Starting controller in STA mode");
    controller.start_async().await.unwrap();
    if let Err(e) = controller.connect_async().await {
        info!("Failed to connect to network: {:?}", e);
        spawn_captive_portal(spawner, rng_seed, interfaces.ap, controller).await;
        return;
    }
    spawner.spawn(connection(controller)).ok();

    let sta_config = embassy_net::Config::dhcpv4(Default::default());
    let (net_stack, net_runner) = embassy_net::new(
        interfaces.sta,
        sta_config,
        make_static!(StackResources::<5>::new()),
        rng_seed,
    );
    spawner.spawn(net_task(net_runner)).ok();
    wait_for_network_ready(net_stack).await;

    let ip_config = net_stack.config_v4().unwrap();
    info!("Got IP {}", ip_config.address);
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match wifi::sta_state() {
            WifiState::StaConnected => {
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after_millis(5000).await;
            }
            _ => {}
        }
        match controller.connect_async().await {
            Ok(_) => info!("reconnected to wifi"),
            Err(e) => {
                error!("failed to reconnect to wifi: {:?}", e);
                Timer::after_millis(5000).await;
            }
        }
    }
}
