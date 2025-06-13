#![no_std]
#![no_main]
#![feature(generic_arg_infer)]
#![feature(type_alias_impl_trait)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::ops::DerefMut;
use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_graphics::mono_font::ascii::{FONT_5X8, FONT_6X12, FONT_9X15_BOLD};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Gray8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;
use embedded_graphics_framebuf::FrameBuf;
use esp_hal::clock::CpuClock;
use esp_hal::dma::DmaChannelFor;
use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig, Pin};
use esp_hal::interrupt::Priority;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::{DMA_CH0, PARL_IO};
use esp_hal::spi::IntoAnySpi;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_embassy::InterruptExecutor;
use esp_println as _;
use matrix_controller_esp32::matrix_parl_io::{DmaFrameBuffer, MatrixParlIo, MatrixParlIoPins};
use static_cell::{make_static, StaticCell};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

type SharedFrameBuf = Mutex<CriticalSectionRawMutex, DmaFrameBuffer>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    // generator version: 0.4.0

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    // let timg0 = TimerGroup::new(peripherals.TIMG0);
    // esp_hal_embassy::init(timg0.timer0);

    info!("Embassy initialized!");

    // let rng = esp_hal::rng::Rng::new(peripherals.RNG);
    // let timer1 = TimerGroup::new(peripherals.TIMG0);
    // let wifi_init = esp_wifi::init(timer1.timer0, rng, peripherals.RADIO_CLK)
    //     .expect("Failed to initialize WIFI/BLE controller");
    // let (mut _wifi_controller, _interfaces) = esp_wifi::wifi::new(&wifi_init, peripherals.WIFI)
    //     .expect("Failed to initialize WIFI controller");

    info!("meow");

    let fbuf = DmaFrameBuffer::new();
    let shared_fb: &SharedFrameBuf = make_static!(Mutex::new(fbuf));

    let sw_ints = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    let software_interrupt = sw_ints.software_interrupt1;

    let hp_executor: &mut InterruptExecutor<1> = make_static!(InterruptExecutor::new(software_interrupt));
    let high_pri_spawner = hp_executor.start(Priority::Priority3);

    high_pri_spawner
        .spawn(matrix(
            MatrixParlIoPins {
                sck: peripherals.GPIO19.degrade(),
                sdo: peripherals.GPIO18.degrade(),
                le_mod: peripherals.GPIO17.degrade(),
                row0: peripherals.GPIO2.degrade(),
                row1: peripherals.GPIO23.degrade(),
                row2: peripherals.GPIO22.degrade(),
                row3: peripherals.GPIO21.degrade(),
            },
            peripherals.PARL_IO,
            peripherals.DMA_CH0,
            shared_fb,
        ))
        .unwrap();
    info!("spawned matrix");

    // spawner.spawn(blink(peripherals.GPIO15.degrade())).unwrap();

    let style = MonoTextStyle::new(&FONT_6X12, Gray8::WHITE);
    let rect_style = PrimitiveStyleBuilder::new().fill_color(Gray8::new(255)).stroke_width(0).build();
    let t = Text::new("wahoo fish!!!", Point::new(1, 14), style);
    let style2 = MonoTextStyle::new(&FONT_6X12, Gray8::new(50));
    let t2 = Text::new("dim text", Point::new(1, 7), style2);

    {
        let mut fb = shared_fb.lock().await;
        t.draw(fb.deref_mut()).unwrap();
        t2.draw(fb.deref_mut()).unwrap();
        // Rectangle::new(Point::new(4, 7), Size::new(8, 1)).into_styled(rect_style).draw(fb.deref_mut()).unwrap();
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
async fn matrix(
    pins: MatrixParlIoPins<'static>,
    parl_io: PARL_IO<'static>,
    dma: DMA_CH0<'static>,
    fb: &'static SharedFrameBuf,
) {
    let mut m = MatrixParlIo::new(parl_io, dma, pins);
    loop {
        {
            // m.render_buffer(fb.lock().await.data.map(|v| v.luma())).await;
            // m.render_buffer(core::array::from_fn(|i| if i % 7 == 0 { 255 } else { 0 })).await;
            // m.render_buffer([255; _]).await;
            let mut fb = fb.lock().await;
            m = m.render(fb.deref_mut()).await.expect("failed to render");
        }
        Timer::after_micros(10).await; // testing
        //                                 Timer::after_millis(10).await;
    }
}
