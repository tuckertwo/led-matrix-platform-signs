#![no_std]
#![no_main]
#![feature(generic_arg_infer)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

use core::ops::DerefMut;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use embedded_graphics::mono_font::ascii::{FONT_5X8, FONT_6X10, FONT_6X12, FONT_6X9};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Gray8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::{LineHeight, Text};
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyleBuilder};
use embedded_text::TextBox;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{AnyPin, Level, Output, OutputConfig, Pin};
use esp_hal::interrupt::Priority;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::peripherals::{DMA_CH0, PARL_IO};
use esp_hal::rng::Rng;
use esp_hal::timer::systimer::SystemTimer;
use esp_hal::timer::timg::TimerGroup;
use esp_hal_embassy::InterruptExecutor;
use esp_println as _;
use matrix_controller_esp32::matrix_parl_io::{DmaFrameBuffer, MatrixParlIo, MatrixParlIoPins};
use static_cell::make_static;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

type SharedFrameBuf = Mutex<CriticalSectionRawMutex, DmaFrameBuffer>;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 96 * 1024);

    let timer0 = SystemTimer::new(peripherals.SYSTIMER);
    esp_hal_embassy::init(timer0.alarm0);

    info!("Embassy initialized!");

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let rng = Rng::new(peripherals.RNG);

    // don't need wifi for bad apple demo - uncomment this line to init the network stack
    // net_init(&spawner, timg0, &mut rng.clone(), peripherals.RADIO_CLK, peripherals.WIFI).await;

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

    spawner.spawn(bad_apple(shared_fb)).unwrap();

    // spawner.spawn(blink(peripherals.GPIO15.degrade())).unwrap();

    let style = MonoTextStyle::new(&FONT_6X10, Gray8::WHITE);
    let textbox_style = TextBoxStyleBuilder::new().alignment(HorizontalAlignment::Center).paragraph_spacing(0).line_height(LineHeight::Percent(90)).build();
    let text_bounds = Rectangle::new(Point::new(0, -1), Size::new(96, 20));
    let textbox = TextBox::with_textbox_style("To S.Waterfront\n1 min & 15 min", text_bounds, style, textbox_style);
    // let rect_style = PrimitiveStyleBuilder::new().fill_color(Gray8::new(255)).stroke_width(0).build();
    // let t = Text::new("To S.Waterfront", Point::new(1, 14), style);
    // let style2 = MonoTextStyle::new(&FONT_5X8, Gray8::new(50));
    // let t2 = Text::new("1 min & 15 min", Point::new(1, 7), style2);

    {
        let mut fb = shared_fb.lock().await;
        textbox.draw(fb.deref_mut()).unwrap();
        // t.draw(fb.deref_mut()).unwrap();
        // t2.draw(fb.deref_mut()).unwrap();
        // Rectangle::new(Point::new(4, 7), Size::new(8, 1)).into_styled(rect_style).draw(fb.deref_mut()).unwrap();
    }
}

#[embassy_executor::task]
async fn blink(pin: AnyPin<'static>) {
    let mut led = Output::new(pin, Level::Low, OutputConfig::default());

    loop {
        info!("blink");
        led.toggle();
        Timer::after_millis(2000).await;
    }
}

const FRAME_W: usize = 96;
const FRAME_H: usize = 16;
const FRAME_COUNT: usize = 2100;
const FPS: u32 = 30;

static BAD_APPLE: &[u8; FRAME_W * FRAME_H * FRAME_COUNT] = include_bytes!("../../bad_apple.rgb");

#[embassy_executor::task]
async fn bad_apple(fb: &'static SharedFrameBuf) {
    let frames = unsafe {
        &*(BAD_APPLE.as_ptr() as *const [[[u8; FRAME_W]; FRAME_H]; FRAME_COUNT])
    };

    Timer::after_secs(3).await;

    loop {
        for i in 0..FRAME_COUNT {
            {
                let mut fb = fb.lock().await;
                for y in 0..FRAME_H {
                    for x in 0..FRAME_W {
                        fb.set_pixel_internal(x, y, Gray8::new(frames[i][y][x]));
                    }
                }
            }
            // technically a bit slow but whatever
            Timer::after(Duration::from_secs(1) / FPS).await;
        }
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
    // we actually don't care about data races, even if an update is half finished we want to
    // render it because we'll get the rest of it soon
    let fb_ptr = unsafe {
        let mut fb = fb.lock().await;
        let p1: *mut DmaFrameBuffer = fb.deref_mut();
        &*p1
    };
    loop {
        m = m.render(fb_ptr).await.expect("failed to render");
        Timer::after_micros(10).await;
    }
}