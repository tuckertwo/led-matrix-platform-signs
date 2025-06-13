use bitfield::bitfield;
use embedded_graphics::pixelcolor::{Gray8};
use embedded_graphics::prelude::*;
use esp_hal::dma::{DmaChannelFor, DmaDescriptor, DmaTxBuf, ReadBuffer};
use esp_hal::dma_descriptors;
use esp_hal::gpio::{AnyPin, NoPin};
use esp_hal::parl_io::{BitPackOrder, ClkOutPin, ParlIo, ParlIoTx, SampleEdge, TxConfig, TxEightBits};
use esp_hal::peripherals::PARL_IO;
use esp_hal::time::Rate;

const ROWS: usize = 16 / 2;
const COLS: usize = 96 * 2;
const BITS: u8 = 3;

// https://github.com/liebman/esp-hub75/blob/8c738d7977f640caebde9b985435b803206586ff/src/framebuffer/mod.rs#L79C5-L81C2
const fn compute_frame_count(bits: u8) -> usize {
    (1usize << bits) - 1
}

const FRAME_COUNT: usize = compute_frame_count(BITS);

bitfield! {
    /// An 8-bit word representing the control signals for a single pixel
    #[derive(Clone, Copy, Default, PartialEq)]
    #[repr(transparent)]
    struct Entry(u8);
    impl Debug;
    unused1, set_unused1: 7;
    unused0, set_unused0: 6;
    value, set_value: 5;
    le_mod, set_le_mod: 4;
    output_blank, set_output_blank: 3;
    row, set_row: 2, 0;
}

impl Entry {
    fn set_color(&mut self, color: Gray8, brightness: u8) {
        self.set_value(color.luma() > brightness);
    }
}

const ROW_EXTRA: usize = 1;

/// Represents a single row of pixels in the framebuffer.
///
/// Each row contains a fixed number of columns (`COLS`) and manages the timing
/// and control signals for the matrix. The row handles:
/// - Output enable timing to prevent ghosting
/// - Latch signal generation for row updates
/// - Row address management
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(C)]
struct Row {
    data: [Entry; COLS + ROW_EXTRA],
}

const BLANKING_DELAY: usize = 25;

impl Row {
    pub fn format(&mut self, addr: u8, prev_addr: u8) {
        let mut entry = Entry(0);
        entry.set_row(prev_addr);
        entry.set_output_blank(true);
        entry.set_le_mod(false);
        for x in 0..COLS {
            // if we enable display too soon then we will have ghosting
            if x == COLS - BLANKING_DELAY - 1 {
                entry.set_output_blank(true);
            } else if x == COLS - 1 {
                entry.set_le_mod(true);
                // entry.set_row(addr);
            } else if x == 1 {
                entry.set_output_blank(false);
            }

            self.data[x] = entry;
        }
        for e in 0..ROW_EXTRA {
            if e == 0 {
                entry.set_row(addr);
                entry.set_le_mod(false);
            }

            if e == ROW_EXTRA - 1 {
                // entry.set_le_mod(false);
                // entry.set_output_blank(false);
            }

            let idx = COLS + e;
            self.data[idx] = entry;
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct Frame {
    rows: [Row; ROWS],
}

impl Frame {
    pub fn format(&mut self) {
        for (addr, row) in self.rows.iter_mut().enumerate() {
            let prev_addr = if addr == 0 { ROWS as u8 - 1 } else { addr as u8 - 1 };
            row.format(addr as u8, prev_addr);
        }
    }

    // works with both types of "rows" (physical and virtual)
    pub fn set_pixel(&mut self, y: usize, x: usize, color: Gray8, brightness: u8) {
        let row = &mut self.rows[if y < ROWS { y } else { y - ROWS }];
        row.data[if y < ROWS { x } else { x + (COLS / 2) }].set_color(color, brightness);
    }
}

type FbFrames = [Frame; FRAME_COUNT];

const fn dma_buffer_size_bytes() -> usize {
    size_of::<FbFrames>()
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct DmaFrameBuffer {
    _align: u64,
    frames: FbFrames
}
impl DmaFrameBuffer {
    pub fn new() -> Self {
        let mut fb = Self {
            _align: 0,
            frames: [Frame { rows: [ Row { data: [Entry(0); _]}; _] }; _]
        };
        fb.clear();
        fb
    }

    pub fn clear(&mut self) {
        for frame in self.frames.iter_mut() {
            frame.format();
        }
    }

    pub fn set_pixel(&mut self, p: Point, color: Gray8) {
        if p.x < 0 || p.y < 0 { return; }
        self.set_pixel_internal(p.x as usize, p.y as usize, color);
    }

    pub fn set_pixel_internal(&mut self, x: usize, y: usize, color: Gray8) {
        if x >= COLS * 2 || y >= ROWS * 2 { return; }
        // set the pixel in all frames
        for i in 0..FRAME_COUNT {
            let brightness_step = 1 << (8 - BITS);
            let brightness = (i as u8 + 1).saturating_mul(brightness_step);
            self.frames[i].set_pixel(y, x, color, brightness);
        }
    }
}

impl OriginDimensions for DmaFrameBuffer {
    fn size(&self) -> Size {
        Size::new((COLS / 2) as u32, (ROWS * 2) as u32)
    }
}

impl DrawTarget for DmaFrameBuffer {
    type Color = Gray8;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where I: IntoIterator<Item = Pixel<Self::Color>> {
        for pixel in pixels {
            self.set_pixel_internal(pixel.0.x as usize, pixel.0.y as usize, pixel.1);
        }
        Ok(())
    }
}

unsafe impl ReadBuffer for DmaFrameBuffer {
    unsafe fn read_buffer(&self) -> (*const u8, usize) {
        let ptr = &self.frames as *const _ as *const u8;
        let len = size_of_val(&self.frames);
        (ptr, len)
    }
}

#[derive(Debug)]
pub enum RenderError {
    ParlIo(esp_hal::parl_io::Error),
    Dma(esp_hal::dma::DmaError)
}

#[derive(Debug)]
pub struct MatrixParlIo<'a> {
    parl_io: ParlIoTx<'a, esp_hal::Async>,
    tx_descriptors: &'static mut [DmaDescriptor]
}

pub struct MatrixParlIoPins<'a> {
    pub sck: AnyPin<'a>,
    pub sdo: AnyPin<'a>,
    pub le_mod: AnyPin<'a>,
    pub row0: AnyPin<'a>,
    pub row1: AnyPin<'a>,
    pub row2: AnyPin<'a>,
    pub row3: AnyPin<'a>,
}

impl<'a> MatrixParlIo<'a> {
    pub fn new(
        parl_io: PARL_IO<'a>,
        dma_channel: impl DmaChannelFor<PARL_IO<'a>>,
        MatrixParlIoPins { sck, sdo, le_mod, row0, row1, row2, row3 }: MatrixParlIoPins<'a>
    ) -> Self {
        let mut idle_value = Entry(0);
        idle_value.set_output_blank(true);
        let config = TxConfig::default()
            .with_frequency(Rate::from_khz(1000))
            .with_idle_value(idle_value.0 as u16) // TODO - this makes a difference, why?? why is it idle
            .with_sample_edge(SampleEdge::Invert)
            .with_bit_order(BitPackOrder::Msb);
        let parl_io = ParlIo::new(parl_io, dma_channel)
            .unwrap()
            .into_async()
            .tx
            .with_config(
                TxEightBits::new(row0, row1, row2, row3, le_mod, sdo, NoPin, NoPin),
                ClkOutPin::new(sck),
                config,
            )
            .unwrap();

        let (_, tx_descriptors) = dma_descriptors!(0, dma_buffer_size_bytes());
        MatrixParlIo {
            parl_io,
            tx_descriptors
        }
    }

    pub async fn render(self, fb: &DmaFrameBuffer) -> Result<Self, (RenderError, Self)> {
        let tx_buffer = unsafe {
            let (ptr, len) = fb.read_buffer();
            core::slice::from_raw_parts_mut(ptr as *mut u8, len)
        };

        let tx_buf = DmaTxBuf::new(self.tx_descriptors, tx_buffer).unwrap();
        let mut xfer = self.parl_io.write(tx_buf.len(), tx_buf).map_err(|(e, parl_io, buf)| {
            let (tx_descriptors, _) = buf.split();
            (RenderError::ParlIo(e), Self { parl_io, tx_descriptors })
        })?;
        xfer.wait_for_done().await;
        let (result, parl_io, tx_buf) = xfer.wait();
        let (tx_descriptors, _) = tx_buf.split();
        let new_matrix = MatrixParlIo { parl_io, tx_descriptors };
        match result {
            Ok(()) => Ok(new_matrix),
            Err(e) => Err((RenderError::Dma(e), new_matrix))
        }
    }
}
