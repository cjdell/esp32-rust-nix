#![feature(async_closure)]
mod audio;
mod auth;
mod common;
mod rfid;
mod speech;
mod wifi;

use audio::AudioService;
use auth::AuthService;
use common::SENDER;
use esp_idf_hal::{cpu::Core, peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, timer::EspTaskTimerService};
use log::{error, info, warn};
use rfid::RfidService;
use speech::SpeechService;
use std::error::Error;
use tokio::sync::mpsc::{self};
use wifi::WifiConnection;

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
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }

    esp_idf_hal::reset::restart();
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    info!("Starting async_main.");

    AudioService::new();

    let speech_service = SpeechService::new();

    speech_service.speak("System Online.".to_owned());

    let (tx, mut rx) = mpsc::channel::<u32>(32);
    unsafe { SENDER = Some(tx.clone()) };

    let rfid_service = RfidService::new(speech_service, tx.clone());

    let event_loop = EspSystemEventLoop::take().unwrap();
    let timer = EspTaskTimerService::new()?;
    let peripherals = peripherals::Peripherals::take().unwrap();
    let nvs_default_partition = nvs::EspDefaultNvsPartition::take().unwrap();

    match esp_idf_hal::cpu::core() {
        Core::Core0 => info!("running on core 0"),
        Core::Core1 => info!("running on core 1"),
    }

    // Initialize the network stack, this must be done before starting the server
    let mut wifi_connection = WifiConnection::new(
        peripherals.modem,
        event_loop,
        timer,
        Some(nvs_default_partition),
        speech_service,
    )
    .await?;

    let auth_service = AuthService::new();

    let mut app_loop = async || -> anyhow::Result<()> {
        while let Some(code) = rx.recv().await {
            println!("==== Code: {:?}", code);

            if let Err(err) = auth_service.check() {
                warn!("Auth failed: {err:?}");
            }
        }

        Ok(())
    };

    tokio::try_join!(
        wifi_connection.connect(),
        rfid_service.run(),
        app_loop(),
        // detect_touch(&speech_service),
    )?;

    Ok(())
}

// async fn detect_touch(speech_service: &SpeechService) -> anyhow::Result<()> {
//     let mut touch = unsafe { PinDriver::input(esp_idf_hal::gpio::Gpio4::new()).unwrap() };
//     touch.set_pull(esp_idf_hal::gpio::Pull::Up)?;

//     loop {
//         if touch.is_high() {
//             // speech_service.speak("Touch.".to_owned());
//             tokio::time::sleep(Duration::from_secs(1)).await;
//         }

//         tokio::time::sleep(Duration::from_millis(100)).await;
//     }
// }
