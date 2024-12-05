use esp_idf_sys::{esp_spiffs_check, esp_vfs_spiffs_conf_t, esp_vfs_spiffs_register, ESP_OK};
use std::{
    ffi::CString,
    fs::{self},
};

pub struct Spiffs {}

impl Spiffs {
    pub fn init() -> anyhow::Result<()> {
        unsafe {
            let label = CString::new("storage").unwrap();
            let base_path = CString::new("/spiffs").unwrap();

            esp_spiffs_check(label.as_ptr());

            let conf = esp_vfs_spiffs_conf_t {
                base_path: base_path.as_ptr(),
                partition_label: label.as_ptr(),
                max_files: 5,
                format_if_mount_failed: false,
            };

            let ret = esp_vfs_spiffs_register(&conf);
            if ret != ESP_OK {
                log::error!("esp_vfs_spiffs_register failed");
            }
        };

        let list = fs::read_dir("/spiffs")?;

        for entry in list {
            let entry = entry?;

            log::info!(
                "Found File: {} {}",
                entry.file_name().into_string().unwrap(),
                entry.metadata().unwrap().len()
            );
        }

        Ok(())
    }

    pub fn read_string(path: String) -> anyhow::Result<String> {
        fs::read_to_string(format!("/spiffs/{}", path))
            .map_err(|err| anyhow::Error::msg(format!("read_string Error: {}", err)))
    }

    pub fn read_binary(path: String) -> anyhow::Result<Vec<u8>> {
        fs::read(format!("/spiffs/{}", path))
            .map_err(|err| anyhow::Error::msg(format!("read_binary Error: {}", err)))
    }

    pub fn write_string(path: String, contents: String) {
        fs::write(format!("/spiffs/{}", path), contents).unwrap();
    }

    pub fn write_binary(path: String, contents: Vec<u8>) {
        fs::write(format!("/spiffs/{}", path), contents).unwrap();
    }
}
