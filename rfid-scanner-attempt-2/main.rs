#![feature(async_closure)]
#![feature(raw_ref_op)]
mod audio;
mod auth;
mod common;
mod rfid;
mod server;
mod speech;
mod spiffs;
mod wifi;

use audio::AudioService;
use auth::AuthService;
use common::SystemMessage;
use esp_idf_hal::{cpu::Core, delay, gpio::PinDriver, peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, ota::EspOta, timer::EspTaskTimerService};
use esp_idf_sys::{printf, sys_delay_ms};
use log::{error, info, warn};
use rfid::RfidService;
use server::HttpServer;
use speech::SpeechService;
use spiffs::Spiffs;
use std::{error::Error, ffi::CString, time::Duration};
use tokio::{sync::mpsc, time::sleep};
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

// #[no_mangle]
// pub extern "C" fn esp_task_wdt_isr_user_handler() {
//     // println!("#### Watchdog triggered! Restarting...");

//     unsafe { printf(CString::new("#### Watchdog triggered! Restarting...").unwrap().into_raw()); };

//     // unsafe { sys_delay_ms(5000) };

//     // esp_idf_svc::hal::reset::restart();
// }

async fn async_main() -> Result<(), Box<dyn Error>> {
    info!("Starting async_main.");

    let (message_bus_tx, mut message_bus_rx) = mpsc::channel::<SystemMessage>(10);

    let mut door = unsafe { PinDriver::output(esp_idf_hal::gpio::Gpio43::new()).unwrap() };
    door.set_high().unwrap();

    Spiffs::init()?;

    let mut http_server = HttpServer::new(message_bus_tx.clone());

    AudioService::new();

    let speech_service = SpeechService::new();

    let ready_msg = Spiffs::read_string("ready.txt".to_string())
        .unwrap_or_else(|err| "Ready message not found".to_string());

    speech_service.speak(ready_msg);

    let rfid_service = RfidService::new(message_bus_tx.clone());

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
        message_bus_tx.clone(),
    )
    .await?;

    let auth_service = AuthService::new(message_bus_tx.clone());

    let mut app_loop = async || -> anyhow::Result<()> {
        loop {
            if let Some(message) = message_bus_rx.recv().await {
                match message {
                    SystemMessage::Speak(str) => {
                        speech_service.speak(str);
                    }
                    SystemMessage::WifiConnected() => {
                        http_server.start().unwrap();

                        speech_service.speak("Connected".to_string());
                    }
                    SystemMessage::OnCard(code) => {
                        println!("==== Code: {:?}", code);

                        if let Err(err) = auth_service.check_text(code).await {
                            warn!("Auth failed: {err:?}");
                        }
                    }
                    SystemMessage::OnAuth(code, name, granted) => {
                        println!("==== Name: {:?}", name);

                        if granted {
                            speech_service.speak(format!("Access granted {}.", name));

                            door.set_low().unwrap();

                            sleep(Duration::from_millis(5000)).await;

                            door.set_high().unwrap();
                        } else {
                            speech_service.speak(format!("Access denied {}.", code));
                        }
                    }
                    SystemMessage::OnOtaBuffer(arc) => {
                        let mut ota = EspOta::new().expect("obtain OTA instance");

                        let mut update = ota.initiate_update().expect("initiate OTA");

                        update.write(&arc).expect("write OTA data");

                        update.complete().expect("complete OTA");

                        esp_idf_svc::hal::reset::restart();
                    }
                }
            }
        }
    };

    tokio::try_join!(wifi_connection.connect(), rfid_service.run(), app_loop(),)?;

    Ok(())
}
