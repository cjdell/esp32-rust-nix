use tokio::sync::mpsc::Sender;

pub const MAX_DELAY: u32 = 0xffffffff;
// pub const WLAN_SSID: &str = "The Lab 2.4GHz";
pub const WLAN_SSID: &str = "49 Grafton Street 2.4GHz";
pub const WLAN_PASS: &str = "Graft0nSt.";

pub static mut SENDER: Option<Sender<u32>> = None;
