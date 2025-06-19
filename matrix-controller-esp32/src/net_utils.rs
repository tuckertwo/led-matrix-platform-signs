use defmt::info;
use embassy_net::{Runner, Stack};
use embassy_time::Timer;
use esp_wifi::wifi::WifiDevice;

pub async fn wait_for_network_ready(net_stack: Stack<'static>) {
    info!("waiting for link up...");
    while !net_stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    info!("waiting for config up...");
    while !net_stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("network ready!");
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}