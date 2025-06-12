#![no_std]
#![no_main]
#![feature(generic_arg_infer)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::ops::DerefMut;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Timer};
use embedded_graphics_framebuf::FrameBuf;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig, Pin};
use esp_hal::spi::{IntoAnySpi};
use esp_hal::timer::systimer::SystemTimer;
use esp_println as _;
use matrix_controller_esp32::matrix::Matrix;
use embassy_sync::mutex::Mutex;
use embedded_graphics::mono_font::ascii::FONT_5X8;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Gray8;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use static_cell::StaticCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

type SharedFrameBuf = Mutex<NoopRawMutex, FrameBuf<Gray8, [Gray8; 96 * 16]>>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.4.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    // let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    // let timer1 = TimerGroup::new(peripherals.TIMG0);
    // let wifi_init = esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK)
    //     .expect("Failed to initialize WIFI/BLE controller");
    // let (mut _wifi_controller, _interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
    //     .expect("Failed to initialize WIFI controller");

    info!("meow");

    static MATRIX: StaticCell<Matrix> = StaticCell::new();
    info!("mrrp");
    let m = MATRIX.init(Matrix::new(
        peripherals.SPI2.degrade(),
        peripherals.DMA_CH0,
        peripherals.GPIO19.degrade(),
        peripherals.GPIO18.degrade(),
        peripherals.GPIO17.degrade(),
        [
            peripherals.GPIO2.degrade(),
            peripherals.GPIO23.degrade(),
            peripherals.GPIO22.degrade(),
            peripherals.GPIO21.degrade(),
        ],
    ));

    info!("init matrix");

    let fbuf = FrameBuf::new([Gray8::BLACK; 96 * 16], 96, 16);
    static SHARED_FB: StaticCell<SharedFrameBuf> = StaticCell::new();
    let shared_fb: &SharedFrameBuf = SHARED_FB.init(Mutex::new(fbuf));

    info!("init shared");


    // spawner.spawn(blink(peripherals.GPIO15.degrade())).unwrap();
    info!("spawned blink");
    spawner.spawn(matrix(m, shared_fb)).unwrap();
    info!("spawned matrix");

    let style = MonoTextStyle::new(&FONT_5X8, Gray8::WHITE);
    let t = Text::new("wahoo fish!", Point::new(0, 0), style);

    {
        let mut fb = shared_fb.lock().await;
        t.draw(fb.deref_mut()).unwrap();
    }

    // loop {
    //     info!("Hello world!");
    //     Timer::after(Duration::from_secs(1)).await;
    // }
}

#[embassy_executor::task]
async fn blink(pin: AnyPin<'static>) {
    let mut led = Output::new(pin, Level::Low, OutputConfig::default());

    loop {
        info!("blink");
        led.toggle();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn matrix(m: &'static mut Matrix<'static>, fb: &'static SharedFrameBuf) {
    loop {
        {
            // m.render_buffer(fb.lock().await.data.map(|v| v.luma())).await;
            m.render_buffer(core::array::from_fn(|i| if i % 7 == 0 { 255 } else { 0 })).await;
            // m.render_buffer([255; _]).await;
        }
        Timer::after_micros(100).await; // TODO: is that a good value?
        // Timer::after_millis(1).await;
    }
}
