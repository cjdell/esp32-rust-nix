use esp_idf_hal::{
    gpio::{Gpio44, Gpio7, Gpio8, Gpio9},
    spi::{self, SpiDriver, SPI3},
};
use mfrc522::Mfrc522;
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::sleep};

use crate::common::SystemMessage;

// #[derive(Copy, Clone)]
pub struct RfidService {
    tx: Sender<SystemMessage>,
}

impl RfidService {
    pub fn new(tx: Sender<SystemMessage>) -> RfidService {
        RfidService { tx }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let sclk = unsafe { Gpio7::new() };
        let sdo = unsafe { Gpio9::new() }; // MOSI
        let sdi = unsafe { Gpio8::new() }; // MISO

        let driver = SpiDriver::new(
            unsafe { SPI3::new() },
            sclk,
            sdo,
            Some(sdi),
            &spi::config::DriverConfig {
                dma: spi::Dma::Disabled,
                intr_flags: enumset::EnumSet::new(),
            },
        )
        .map_err(|err| anyhow::Error::msg(format!("SpiDriver Error: {}", err)))?;

        let spi_device_driver = spi::SpiDeviceDriver::new(
            driver,
            Some(unsafe { Gpio44::new() }),
            &esp_idf_hal::spi::config::Config::new(),
        )
        .map_err(|err| anyhow::Error::msg(format!("SpiDeviceDriver Error: {}", err)))?;

        let itf = mfrc522::comm::blocking::spi::SpiInterface::new(spi_device_driver);

        let mut mfrc522 = Mfrc522::new(itf).init().unwrap();

        let vers = mfrc522
            .version()
            .map_err(|err| anyhow::Error::msg(format!("mfrc522.version Error: {:?}", err)))?;

        println!("VERSION: 0x{:x}", vers);

        loop {
            if let Ok(atqa) = mfrc522.reqa() {
                if let Ok(uid) = mfrc522.select(&atqa) {
                    let bytes = uid.as_bytes();

                    println!("UID: {:?}", uid.as_bytes());
                    println!("Number: {}", to_u32(bytes).unwrap_or_default());

                    let code = to_u32(bytes).unwrap_or_default();

                    self.tx.send(SystemMessage::OnCard(code)).await?;

                    // Don't spam
                    sleep(Duration::from_secs(3)).await;
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }
}

fn to_u32(bytes: &[u8]) -> Option<u32> {
    // Ensure the slice has exactly 4 bytes
    if bytes.len() == 4 {
        // Convert bytes to u32 assuming little-endian
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    } else {
        None
    }
}
