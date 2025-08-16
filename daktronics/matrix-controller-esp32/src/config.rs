use core::str::FromStr;
use defmt::{info, warn};
use embedded_storage::{ReadStorage, Storage};
use esp_storage::{FlashStorage, FlashStorageError};
use heapless::String;

const FLASH_BASE_ADDRESS: u32 = 0x9000;
pub const CONFIG_ENTRY_LEN: usize = 64;

pub const SSID_STORE_ID: u32 = 0;
pub const PW_STORE_ID: u32 = 1;

pub struct ConfigStore {
    storage: FlashStorage,
}

impl ConfigStore {
    pub fn new() -> Self {
        Self {
            storage: FlashStorage::new(),
        }
    }

    fn offset_of_id(id: u32) -> u32 {
        FLASH_BASE_ADDRESS + (id * CONFIG_ENTRY_LEN as u32)
    }

    pub fn set(&mut self, id: u32, value: &str) -> Result<(), FlashStorageError> {
        if value.len() > CONFIG_ENTRY_LEN {
            return Err(FlashStorageError::Other(0));
        }

        let mut buf = [0; CONFIG_ENTRY_LEN];
        buf[..value.len()].copy_from_slice(value.as_bytes());
        let offset = Self::offset_of_id(id);
        self.storage.write(offset, buf.as_slice())
    }

    pub fn get(&mut self, id: u32) -> Result<String<CONFIG_ENTRY_LEN>, FlashStorageError> {
        let mut buf = [0; CONFIG_ENTRY_LEN];
        let offset = Self::offset_of_id(id);
        self.storage.read(offset, &mut buf)?;
        let s: String<CONFIG_ENTRY_LEN> =
            String::from_str(str::from_utf8(&buf).unwrap_or_else(|_| {
                warn!("failed to convert flash contents to utf-8");
                ""
            }).trim_matches(char::from(0)))
                .map_err(|_| FlashStorageError::Other(0))?;
        Ok(s)
    }
}
