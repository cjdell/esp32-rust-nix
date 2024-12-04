pub const MAX_DELAY: u32 = 0xffffffff;
// pub const WLAN_SSID: &str = "The Lab 2.4GHz";
// pub const WLAN_SSID: &str = "49 Grafton Street 2.4GHz";
// pub const WLAN_PASS: &str = "Graft0nSt.";
pub const WLAN_SSID: &str = "Leighhack";
pub const WLAN_PASS: &str = "caffeine1234";

#[derive(Clone, Debug)]
pub enum SystemMessage {
    Speak(String),
    WifiConnected(),
    OnCard(u32),
    OnAuth(String, bool),
}
