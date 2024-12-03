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
use esp_idf_hal::{cpu::Core, gpio::PinDriver, peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, timer::EspTaskTimerService};
use log::{error, info, warn};
use rfid::RfidService;
use speech::SpeechService;
use std::{error::Error, time::Duration};
use tokio::sync::mpsc::{self, Sender};
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
    tokio::time::sleep(Duration::from_secs(3)).await;

    info!("Starting async_main.");

    let audio_service = AudioService::new();

    tokio::time::sleep(Duration::from_secs(3)).await;

    let speech_service = SpeechService::new(audio_service);

    // speak("Avoid repeatedly calculating indices. We can use the copy_from_slice method, which copies data in bulk rather than assigning individual elements. Reduce pointer arithmetic in the loop: We can directly iterate over the buffer as a slice. Minimize temporary variables: Directly calculate bytes without assigning it to a temporary variable. Make the stretched_buffer initialization more efficient by filling sections at a time rather than manually assigning individual indices.".to_owned());
    speech_service.speak("System Online.".to_owned());

    tokio::time::sleep(Duration::from_secs(5)).await;

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

            // if let Err(err) = auth_service.check1().await {
            //     warn!("Auth failed: {err:?}");
            // }

            // tokio::time::sleep(Duration::from_secs(5)).await;

            if let Err(err) = auth_service.check2() {
                warn!("Auth failed: {err:?}");
            }
        }

        Ok(())
    };

    tokio::try_join!(
        // audio_service.run(),
        wifi_connection.connect(),
        rfid_service.run(),
        app_loop(),
        // detect_touch(&speech_service),
    )?;

    Ok(())
}

// async fn detect_touch(speech_service: &SpeechService) -> anyhow::Result<()> {
//     let mut touch = unsafe { PinDriver::input(esp_idf_hal::gpio::Gpio1::new()).unwrap() };
//     touch.set_pull(esp_idf_hal::gpio::Pull::Up)?;

//     loop {
//         if touch.is_high() {
//             // speech_service.speak("Touch.".to_owned());
//             tokio::time::sleep(Duration::from_secs(1)).await;
//         }

//         tokio::time::sleep(Duration::from_millis(100)).await;
//     }
// }
