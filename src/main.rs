mod audio;
mod common;
mod wifi;

use anyhow::Result;
use audio::write_samples_directly;
use esp_idf_hal::{
    gpio::PinDriver,
    peripherals,
    spi::{self, SPI3},
};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, timer::EspTaskTimerService};
use esp_idf_sys::{picotts_add, picotts_init, vTaskDelay, xRingbufferSend};
use log::{error, info};
use mfrc522::Mfrc522;
use std::{
    ffi::{c_void, CString},
    thread::{self, sleep},
    time::Duration,
};
use wifi::WifiConnection;

const TTS_CORE: i32 = 1;
const TTS_PRI: u32 = 20;

static mut SENT_CHUNKS: usize = 0;
static mut SENT_BYTES: usize = 0;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    esp_idf_svc::io::vfs::initialize_eventfd(1).expect("Failed to initialize eventfd");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    match rt.block_on(async { async_main().await }) {
        Ok(()) => info!("main() finished, reboot."),
        Err(err) => {
            error!("{err:?}");
            // Let them read the error message before rebooting
            sleep(std::time::Duration::from_secs(3));
        }
    }

    esp_idf_hal::reset::restart();
}

async fn async_main() -> Result<()> {
    sleep(Duration::from_secs(3));
    info!("Starting async_main.");

    audio::init_audio();

    unsafe {
        picotts_init(TTS_PRI, Some(on_samples), TTS_CORE);
    }

    // speak("Avoid repeatedly calculating indices. We can use the copy_from_slice method, which copies data in bulk rather than assigning individual elements. Reduce pointer arithmetic in the loop: We can directly iterate over the buffer as a slice. Minimize temporary variables: Directly calculate bytes without assigning it to a temporary variable. Make the stretched_buffer initialization more efficient by filling sections at a time rather than manually assigning individual indices.".to_owned());
    speak("System Online.".to_owned());
    sleep(Duration::from_secs(5));

    // let handle = thread::spawn(|| unsafe {
    //     card_reader().unwrap_or_else(|err| println!("card_reader: {}", err));
    // });

    thread::Builder::new()
        .stack_size(8192)
        .name("Card Reader Thread".to_string())
        .spawn(|| unsafe {
            card_reader().unwrap_or_else(|err| println!("card_reader: {}", err));
        })
        .unwrap();

    thread::Builder::new()
        .stack_size(8192)
        .name("Touch Thread".to_string())
        .spawn(|| unsafe {
            detect_touch();
        })
        .unwrap();

    let event_loop = EspSystemEventLoop::take().unwrap();
    let timer = EspTaskTimerService::new()?;
    let peripherals = peripherals::Peripherals::take().unwrap();
    let nvs_default_partition = nvs::EspDefaultNvsPartition::take().unwrap();

    // let mut wifi =
    //     EspWifi::new(peripherals.modem, event_loop, Some(nvs_default_partition)).unwrap();

    // wifi.start().unwrap();

    // speak("Searching for WiFi networks.".to_owned());

    // let scan_result = wifi.scan().unwrap();

    // sleep(Duration::from_secs(2));

    // speak("WiFi networks detected.".to_owned());

    // let mut i = 0;
    // for line in scan_result {
    //     speak(format!(
    //         "{}, signal strength {}.",
    //         line.ssid, line.signal_strength
    //     ));
    //     i += 1;
    //     if i == 3 {
    //         break;
    //     };
    // }

    // let client_config = ClientConfiguration {
    //     ssid: heapless::String::from_str("Leighhack").unwrap(),
    //     password: heapless::String::from_str("caffeine1234").unwrap(),
    //     ..Default::default()
    // };

    // wifi.set_configuration(&Configuration::Client(client_config))
    //     .unwrap();

    // speak("Connecting.".to_owned());

    // wifi.connect().unwrap();

    // sleep(Duration::from_secs(5));

    // speak("Connected.".to_owned());

    // let ip_info = wifi.sta_netif().get_ip_info();

    // let ip = ip_info.ok().map(|i| i.ip);

    // speak(format!(
    //     "IP address: {}.",
    //     ip.unwrap().to_string().replace(".", " dot ")
    // ));

    // Initialize the network stack, this must be done before starting the server
    let mut wifi_connection = WifiConnection::new(
        peripherals.modem,
        event_loop,
        timer,
        Some(nvs_default_partition),
    )
    .await?;

    tokio::try_join!(
        // run_server(wifi_connection.state.clone()),
        wifi_connection.connect()
    )?;

    Ok(())

    // let mut counter = 0;

    // loop {
    //     // speak(format!(
    //     //     "Hello world. This is iteration number {}.",
    //     //     counter
    //     // ));
    //     sleep(Duration::from_secs(5));
    //     counter += 1;

    //     unsafe {
    //         log::info!(
    //             "Counter: {} SENT:{}/{} RECV:{}/{}",
    //             counter,
    //             SENT_CHUNKS,
    //             SENT_BYTES,
    //             RECV_CHUNKS,
    //             RECV_BYTES
    //         );
    //     };
    // }
}

unsafe fn detect_touch() {
    let touch = PinDriver::input(esp_idf_hal::gpio::Gpio1::new()).unwrap();

    loop {
        if touch.is_high() {
            speak("Touch.".to_owned());
            sleep(Duration::from_secs(1));
        }

        sleep(Duration::from_millis(10));
    }
}

unsafe fn card_reader() -> Result<(), String> {
    let sclk = esp_idf_hal::gpio::Gpio7::new();
    let sdo = esp_idf_hal::gpio::Gpio9::new(); // MOSI
    let sdi = esp_idf_hal::gpio::Gpio8::new(); // MISO

    let driver = spi::SpiDriver::new(
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

                speak(format!("Card {}.", to_u32(bytes).unwrap_or_default()));

                // Don't spam
                sleep(Duration::from_secs(3));
            }
        }

        sleep(Duration::from_millis(100));
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

fn speak(str: String) {
    let lines = str.split(".");

    for line in lines {
        let line = line.trim().to_owned() + ". "; // Won't start speaking until a space is seen after a full stop.

        log::info!("{}", line);

        let len = line.len() as u32;
        let c_str = CString::new(line).unwrap();

        unsafe {
            picotts_add(c_str.as_ptr(), len);
            vTaskDelay(len * 100);
        };
    }
}

unsafe extern "C" fn on_samples(buffer: *mut i16, length: u32) {
    // let factor = 3;
    let length = length as usize;

    // write_samples_directly(buffer, length);

    // // Convert the raw pointer to a slice for safer and more efficient access
    // let input_slice = std::slice::from_raw_parts(buffer, length);

    // // Create a new vector with the expanded size
    // let mut stretched_buffer = vec![0i16; length * factor];

    // // Fill the stretched buffer by copying each sample `factor` times
    // for (i, &sample) in input_slice.iter().enumerate() {
    //     let start_idx = i * factor;
    //     stretched_buffer[start_idx..start_idx + factor].fill(sample);
    // }

    // // Cast the stretched buffer to a *const c_void
    // let c_buffer: *const c_void = stretched_buffer.as_ptr() as *const c_void;

    // // Calculate the number of bytes to send and update sent
    // let bytes = stretched_buffer.len() * std::mem::size_of::<i16>();

    let c_buffer = buffer as *const c_void;
    let bytes = length * std::mem::size_of::<i16>();

    SENT_CHUNKS += 1;
    SENT_BYTES += bytes;

    // Send to the ring buffer
    // print!("I");
    xRingbufferSend(audio::RING_BUF, c_buffer, bytes, common::MAX_DELAY);

    // Stops the watch guard timer from killing the task (I think...)
    if SENT_CHUNKS % 100 == 0 {
        vTaskDelay(1);
    }

    // let mut bytes_written: usize = 0;
    // let ret = i2s_channel_write(i2s_tx_chan, c_void_ptr, bytes, &mut bytes_written, 100);
    // if ret != ESP_OK {
    //     log::error!("i2s_channel_write failed");
    // }
}
