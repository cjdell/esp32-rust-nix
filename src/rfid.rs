use crate::speech::SpeechService;
use esp_idf_hal::{
    gpio::{self, Gpio7, Gpio8, Gpio9},
    spi::{self, SpiDriver, SPI3},
};
use mfrc522::Mfrc522;
use std::{
    error::Error,
    thread::{self, sleep},
    time::Duration,
};

#[derive(Copy, Clone)]
pub struct RfidService {
    speech_service: SpeechService,
}

impl RfidService {
    pub fn new(speech_service: SpeechService) -> RfidService {
        RfidService { speech_service }
    }

    pub fn run(self) -> Result<(), Box<dyn Error>> {
        thread::Builder::new()
            .stack_size(8192)
            .name("Card Reader Thread".to_string())
            .spawn(move || unsafe {
                self._run()
                    .unwrap_or_else(|err| println!("card_reader: {}", err));
            })
            .map_err(|err| format!("RfidService Thread Start Error: {:?}", err))?;

        Ok(())
    }

    unsafe fn _run(&self) -> Result<(), Box<dyn Error>> {
        let sclk = Gpio7::new();
        let sdo = Gpio9::new(); // MOSI
        let sdi = Gpio8::new(); // MISO

        let driver = SpiDriver::new(
            SPI3::new(),
            sclk,
            sdo,
            Some(sdi),
            &spi::config::DriverConfig {
                dma: spi::Dma::Disabled,
                intr_flags: enumset::EnumSet::new(),
            },
        )
        .map_err(|err| format!("SpiDriver Error: {}", err))?;

        // let spi_bus_driver = spi::SpiBusDriver::new(driver, &esp_idf_hal::spi::config::Config::new());

        let spi_device_driver = spi::SpiDeviceDriver::new(
            driver,
            Some(esp_idf_hal::gpio::Gpio43::new()),
            &esp_idf_hal::spi::config::Config::new(),
        )
        .map_err(|err| format!("SpiDeviceDriver Error: {}", err))?;

        let itf = mfrc522::comm::blocking::spi::SpiInterface::new(spi_device_driver);

        let mut mfrc522 = Mfrc522::new(itf).init().unwrap();

        let vers = mfrc522
            .version()
            .map_err(|err| format!("mfrc522.version Error: {:?}", err))?;

        println!("VERSION: 0x{:x}", vers);

        loop {
            if let Ok(atqa) = mfrc522.reqa() {
                if let Ok(uid) = mfrc522.select(&atqa) {
                    let bytes = uid.as_bytes();

                    println!("UID: {:?}", uid.as_bytes());
                    println!("Number: {}", to_u32(bytes).unwrap_or_default());

                    self.speech_service
                        .speak(format!("Card {}.", to_u32(bytes).unwrap_or_default()));

                    // Don't spam
                    sleep(Duration::from_secs(3));
                }
            }

            sleep(Duration::from_millis(100));
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
