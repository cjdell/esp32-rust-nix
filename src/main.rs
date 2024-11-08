mod audio;
mod common;
mod rfid;
mod speech;
mod wifi;

use esp_idf_hal::{gpio::PinDriver, peripherals};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs, timer::EspTaskTimerService};
use log::{error, info};
use rfid::RfidService;
use speech::SpeechService;
use std::{
    error::Error,
    thread::{self, sleep},
    time::Duration,
};
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
            sleep(std::time::Duration::from_secs(3));
        }
    }

    esp_idf_hal::reset::restart();
}

async fn async_main() -> Result<(), Box<dyn Error>> {
    sleep(Duration::from_secs(3));
    info!("Starting async_main.");

    audio::init_audio();

    let speech_service = SpeechService::new();

    // speak("Avoid repeatedly calculating indices. We can use the copy_from_slice method, which copies data in bulk rather than assigning individual elements. Reduce pointer arithmetic in the loop: We can directly iterate over the buffer as a slice. Minimize temporary variables: Directly calculate bytes without assigning it to a temporary variable. Make the stretched_buffer initialization more efficient by filling sections at a time rather than manually assigning individual indices.".to_owned());
    speech_service.speak("System Online.".to_owned());

    sleep(Duration::from_secs(5));

    let rfid_service = RfidService::new(speech_service);

    rfid_service.run()?;

    thread::Builder::new()
        .stack_size(8192)
        .name("Touch Thread".to_string())
        .spawn(move || unsafe {
            detect_touch(&speech_service);
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
}

unsafe fn detect_touch(speech_service: &SpeechService) {
    let touch = PinDriver::input(esp_idf_hal::gpio::Gpio1::new()).unwrap();

    loop {
        if touch.is_high() {
            speech_service.speak("Touch.".to_owned());
            sleep(Duration::from_secs(1));
        }

        sleep(Duration::from_millis(10));
    }
}
