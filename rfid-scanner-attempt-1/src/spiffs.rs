use esp_idf_sys::{esp_spiffs_check, esp_vfs_spiffs_conf_t, esp_vfs_spiffs_register, ESP_OK};
use std::{
    ffi::CString,
    fs::{self, File},
    io::Read,
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

        // match fs::read_to_string("/spiffs/ready.txt") {
        //     Ok(str) => {
        //         log::info!("READ {}", str);
        //     },
        //     Err(err) => {
        //         log::error!("read_string failed: {}", err);
        //     }
        // };

        Ok(())
    }

    pub fn read_string(path: String) -> String {
        match fs::read_to_string(format!("/spiffs/{}", path)) {
            Ok(str) => str,
            Err(err) => {
                log::error!("read_string failed: {}", err);
                "".to_string()
            }
        }

        // match File::open("/spiffs/test_write.txt") {
        //     Ok(mut file) => {
        //         let mut s: String = String::new();

        //         match file.read_to_string(&mut s) {
        //             Ok(len) => {
        //                 log::error!("read {} bytes", len);

        //                 s
        //             }
        //             Err(err) => {
        //                 log::error!("read failed: {}", err);
        //                 "".to_string()
        //             }
        //         }
        //     }
        //     Err(err) => {
        //         log::error!("open failed: {}", err);
        //         "".to_string()
        //     }
        // }
    }

    pub fn write_string(path: String, contents: String) {
        fs::write(format!("/spiffs/{}", path), contents).unwrap();
    }
}
