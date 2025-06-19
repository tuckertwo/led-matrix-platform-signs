// old spi driver
// not used anymore but kept around for reference
// might help you understand how the parl_io driver works!

use core::array;
use embassy_time::Timer;
use esp_hal::dma::{DmaChannelFor, DmaRxBuf, DmaTxBuf};
use esp_hal::dma_buffers;
use esp_hal::gpio::{AnyPin, DriveMode, Level, Output, OutputConfig};
use esp_hal::spi::master::{Config, Spi, SpiDmaBus};
use esp_hal::spi::{AnySpi, Mode};
use esp_hal::time::Rate;

pub struct Matrix<'a> {
    spi: SpiDmaBus<'a, esp_hal::Async>,
    le_mod: Output<'a>,
    rows: [Output<'a>; 4],
    current_row: u8,
}

fn level_of_u8(n: u8) -> Level {
    if n == 0 {
        Level::Low
    } else {
        Level::High
    }
}

impl<'a> Matrix<'a> {
    pub fn new(
        spi: AnySpi<'a>,
        dma_channel: impl DmaChannelFor<AnySpi<'a>>,
        sck: AnyPin<'a>,
        sdo: AnyPin<'a>,
        le_mod: AnyPin<'a>,
        rows: [AnyPin<'a>; 4],
    ) -> Self {
        // we're not using rx dma (nothing to receive), but the spi api makes us make a buffer anyway
        // it fails if we make the rx buffer size 0, so we have to set it to 1
        let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(1, 192 / 8);
        let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();
        let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();

        let spi = Spi::new(
            spi,
            Config::default()
                .with_frequency(Rate::from_khz(100))
                .with_mode(Mode::_3), // to send bit, pull clock low and set sdo on falling edge
        )
        .unwrap()
        .with_sck(sck)
        .with_mosi(sdo)
        .with_dma(dma_channel)
        .with_buffers(dma_rx_buf, dma_tx_buf)
        .into_async();

        Self {
            spi,
            le_mod: Output::new(
                le_mod,
                Level::High,
                OutputConfig::default().with_drive_mode(DriveMode::PushPull),
            ),
            rows: rows.map(|row| Output::new(row, Level::High, OutputConfig::default())),
            current_row: 0,
        }
    }

    async fn render_row(&mut self, n: u8, data: &[u8; 192 / 8]) {
        self.le_mod.set_low();
        self.spi.write_async(data).await.unwrap();
        self.rows[3].set_high();
        self.rows[0].set_level(level_of_u8(n % 2));
        self.rows[1].set_level(level_of_u8(n / 2 % 2));
        self.rows[2].set_level(level_of_u8(n / 4 % 2));
        self.le_mod.set_high();
        Timer::after_micros(50).await;
        self.rows[3].set_low();
    }

    // render the next two physical rows of the display
    pub async fn render_buffer(&mut self, buffer: [u8; 96 * 16]) {
        let buffer_pos: usize = (self.current_row * 192) as usize;
        // used to have a t*do for brightness support but this driver is just being kept around
        // for documentation purposes now
        let row_data: [u8; 192 / 8] = array::from_fn(|i| {
            buffer[(buffer_pos + i * 8)..(buffer_pos + (i + 1) * 8)]
                .iter()
                .fold(0u8, |acc, cur| (acc << 1) | (*cur > 127) as u8)
        });
        self.render_row(self.current_row, &row_data).await;
        self.current_row = (self.current_row + 1) % 8;
    }
}
