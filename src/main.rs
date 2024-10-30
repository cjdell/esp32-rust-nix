use esp_idf_hal::spi;
use esp_idf_hal::spi::SPI3;
// use esp_idf_hal::{gpio::*, i2s::config::*, i2s::*, peripherals::*};
use esp_idf_sys::*;
use esp_idf_sys::{picotts_add, picotts_init};
use mfrc522::Mfrc522;
use std::ffi::c_void;
use std::{ffi::CString, thread::sleep, time::Duration};

static mut I2S_TX_CHAN: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

static mut RING_BUF: RingbufHandle_t = 0 as RingbufHandle_t;

const MAX_DELAY: u32 = 0xffffffff;
const BUFFER_SIZE_SAMPLES: usize = 2048;

const TTS_CORE: i32 = 1;
const TTS_PRI: u32 = 20;

const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

static mut SENT_CHUNKS: usize = 0;
static mut SENT_BYTES: usize = 0;
static mut RECV_CHUNKS: usize = 0;
static mut RECV_BYTES: usize = 0;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    sleep(Duration::from_secs(3));

    // unsafe {
    //     let i2s_driver = I2sDriver::new_pdm_tx(
    //         I2S0::new(),
    //         &PdmTxConfig::new(
    //             I2sConfig::new(),
    //             PdmTxClkConfig::from_sample_rate_hz(48000),
    //             PdmTxSlotConfig::from_slot_mode(SlotMode::Mono),
    //             PdmTxGpioConfig::new(false),
    //         ),
    //         esp_idf_hal::gpio::Gpio45::new(),
    //         esp_idf_hal::gpio::Gpio44::new(),
    //         None,
    //     );
    // };

    let chan_cfg = i2s_chan_config_t {
        id: i2s_port_t_I2S_NUM_0,
        role: i2s_role_t_I2S_ROLE_MASTER,
        dma_desc_num: 6,
        dma_frame_num: 240,
        auto_clear: false,
        intr_priority: 0,
    };

    unsafe {
        let null = 0 as *mut *mut i2s_channel_obj_t;

        let ret = i2s_new_channel(&chan_cfg, &raw mut I2S_TX_CHAN, null);
        if ret != ESP_OK {
            log::error!("i2s_new_channel failed");
        }
    }

    let mut invert_flags = i2s_pdm_tx_gpio_config_t__bindgen_ty_1::default();
    invert_flags.set_clk_inv(0);

    let pdm_tx_cfg = i2s_pdm_tx_config_t {
        clk_cfg: i2s_pdm_tx_clk_config_t {
            sample_rate_hz: 16000,
            clk_src: soc_periph_i2s_clk_src_t_I2S_CLK_SRC_DEFAULT,
            mclk_multiple: i2s_mclk_multiple_t_I2S_MCLK_MULTIPLE_256,
            up_sample_fp: 960,
            up_sample_fs: 480,
            bclk_div: 8,
        },
        slot_cfg: i2s_pdm_tx_slot_config_t {
            data_bit_width: i2s_data_bit_width_t_I2S_DATA_BIT_WIDTH_16BIT,
            slot_bit_width: i2s_slot_bit_width_t_I2S_SLOT_BIT_WIDTH_16BIT,
            slot_mode: i2s_slot_mode_t_I2S_SLOT_MODE_MONO,
            sd_prescale: 0,
            sd_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
            hp_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_DIV_2,
            lp_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
            sinc_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
            line_mode: i2s_pdm_tx_line_mode_t_I2S_PDM_TX_ONE_LINE_CODEC,
            hp_en: true,
            hp_cut_off_freq_hz: 35.5,
            sd_dither: 0,
            sd_dither2: 1,
        },
        gpio_cfg: i2s_pdm_tx_gpio_config_t {
            clk: gpio_num_t_GPIO_NUM_45,
            dout: gpio_num_t_GPIO_NUM_44,
            dout2: 0,
            invert_flags,
        },
    };

    unsafe {
        let ret = i2s_channel_init_pdm_tx_mode(I2S_TX_CHAN, &pdm_tx_cfg);
        if ret != ESP_OK {
            log::error!("i2s_channel_init_pdm_tx_mode failed");
        }
    }

    unsafe {
        let ret = i2s_channel_enable(I2S_TX_CHAN);
        if ret != ESP_OK {
            log::error!("i2s_channel_enable failed");
        }
    }

    unsafe {
        RING_BUF = xRingbufferCreate(
            BUFFER_SIZE_SAMPLES * std::mem::size_of::<i16>(),
            RingbufferType_t_RINGBUF_TYPE_BYTEBUF,
        );

        let task_name = CString::new("I2SWriteTask").unwrap();

        xTaskCreatePinnedToCore(
            Some(i2s_write_task),
            task_name.as_ptr(),
            2048,
            (0) as *mut c_void,
            I2S_PRI,
            (0) as *mut TaskHandle_t,
            I2S_CORE,
        )
    };

    unsafe {
        picotts_init(TTS_PRI, Some(on_samples), TTS_CORE);
    }

    // speak("Avoid repeatedly calculating indices. We can use the copy_from_slice method, which copies data in bulk rather than assigning individual elements. Reduce pointer arithmetic in the loop: We can directly iterate over the buffer as a slice. Minimize temporary variables: Directly calculate bytes without assigning it to a temporary variable. Make the stretched_buffer initialization more efficient by filling sections at a time rather than manually assigning individual indices.".to_owned());
    speak("System Online.".to_owned());
    sleep(Duration::from_secs(5));

    unsafe {
        card_reader().unwrap_or_else(|err| println!("card_reader: {}", err));
    }

    let mut counter = 0;

    loop {
        speak(format!(
            "Hello world. This is iteration number {}.",
            counter
        ));
        sleep(Duration::from_secs(5));
        counter += 1;

        unsafe {
            log::info!(
                "SENT:{}/{} RECV:{}/{}",
                SENT_CHUNKS,
                SENT_BYTES,
                RECV_CHUNKS,
                RECV_BYTES
            );
        };
    }
}

unsafe fn card_reader() -> Result<(), String> {
    // let cs = PinDriver::output(esp_idf_hal::gpio::Gpio5::new()).unwrap();

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

unsafe extern "C" fn i2s_write_task(_param: *mut c_void) {
    let mut item_size: usize = 0;

    loop {
        let buffer = xRingbufferReceive(RING_BUF, &mut item_size, MAX_DELAY);

        if buffer != 0 as *mut c_void {
            let mut bytes_written: usize = 0;

            RECV_CHUNKS += 1;
            RECV_BYTES += item_size;

            // print!("O");
            let ret = i2s_channel_write(
                I2S_TX_CHAN,
                buffer,
                item_size,
                &mut bytes_written,
                MAX_DELAY,
            );
            if ret != ESP_OK {
                log::error!("i2s_channel_write failed");
            }

            vRingbufferReturnItem(RING_BUF, buffer);

            vTaskDelay(10);
        }
    }
}

unsafe extern "C" fn on_samples(buffer: *mut i16, length: u32) {
    // let factor = 3;
    let length = length as usize;

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
    xRingbufferSend(RING_BUF, c_buffer, bytes, MAX_DELAY);

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
