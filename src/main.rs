use esp_idf_sys::picotts_init;
use std::{thread::sleep, time::Duration};

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    unsafe {
        picotts_init(0, Some(foo), 0);
    }

    loop {
        log::info!("Hello, world!");

        sleep(Duration::from_secs(1));
    }
}

unsafe extern "C" fn foo(a: *mut i16, b: u32) {
    //
}
