use esp_idf_hal::{gpio::*, i2s::config::*, i2s::*, peripherals::*};
use esp_idf_sys::*;
use esp_idf_sys::{picotts_add, picotts_init};
use std::ffi::c_void;
use std::{ffi::CString, thread::sleep, time::Duration};

static mut i2s_tx_chan: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

static mut ringbuf: RingbufHandle_t = (0 as RingbufHandle_t);

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    sleep(Duration::from_secs(3));

    // let i2s_tx_chan: *mut i2s_chan_handle_t = 0 as *mut i2s_chan_handle_t;
    // unsafe { i2s_tx_chan = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t };

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

        let ret = i2s_new_channel(&chan_cfg, &mut i2s_tx_chan, null);
        if ret != ESP_OK {
            log::error!("i2s_new_channel failed");
        }
    }

    let mut invert_flags = i2s_pdm_tx_gpio_config_t__bindgen_ty_1::default();
    invert_flags.set_clk_inv(0);

    let pdm_tx_cfg = i2s_pdm_tx_config_t {
        clk_cfg: i2s_pdm_tx_clk_config_t {
            sample_rate_hz: 48000,
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
        let ret = i2s_channel_init_pdm_tx_mode(i2s_tx_chan, &pdm_tx_cfg);
        if ret != ESP_OK {
            log::error!("i2s_channel_init_pdm_tx_mode failed");
        }
    }

    unsafe {
        let ret = i2s_channel_enable(i2s_tx_chan);
        if ret != ESP_OK {
            log::error!("i2s_channel_enable failed");
        }
    }

    unsafe {
        picotts_init(10, Some(foo), 1);
    }

    unsafe { ringbuf = xRingbufferCreate(1024 * 2, RingbufferType_t_RINGBUF_TYPE_BYTEBUF) };

    unsafe {
        xTaskCreatePinnedToCore(
            Some(i2s_write_task),
            CString::new("I2SWriteTask").unwrap().as_ptr(),
            2048,
            (0) as *mut c_void,
            1,
            (0) as *mut TaskHandle_t,
            1,
        )
    };

    loop {
        log::info!("Hello, world!");

        sleep(Duration::from_secs(5));

        unsafe {
            let my_string = "Hello World.   ";
            // Convert &str to CString
            let c_string = CString::new(my_string).expect("CString::new failed");

            // Convert CString to *const i8
            let c_string_ptr: *const i8 = c_string.as_ptr();

            picotts_add(c_string_ptr, 14);
        }
    }
}

unsafe extern "C" fn i2s_write_task(param: *mut c_void) {
    let mut item_size: usize = 0;

    loop {
        let buffer = xRingbufferReceive(ringbuf, &mut item_size, 100);

        if (buffer != 0 as *mut c_void) {
            let mut bytes_written: usize = 0;

            let ret = i2s_channel_write(i2s_tx_chan, buffer, item_size, &mut bytes_written, 100);
            if ret != ESP_OK {
                log::error!("i2s_channel_write failed");
            }

            vRingbufferReturnItem(ringbuf, buffer);
        }

        vTaskDelay(1);
    }
}

unsafe extern "C" fn on_samples(buffer: *mut i16, length: u32) {
    // log::info!("{} ", length);

    let mut bytes_written: usize = 0;

    let mut buffer2 = vec![0i16; (length * 3) as usize];

    let mut b = buffer;
    for i in 0..length {
        buffer2[(i * 3 + 0) as usize] = *b;
        buffer2[(i * 3 + 1) as usize] = *b;
        buffer2[(i * 3 + 2) as usize] = *b;

        b = b.wrapping_add(1);
    }

    // Get a raw pointer to the vector's data
    let ptr: *const i16 = buffer2.as_ptr();

    // Cast the pointer to `*const c_void`
    let c_void_ptr: *const c_void = ptr as *const c_void;

    xRingbufferSend(ringbuf, c_void_ptr, (length * 3 * 2) as usize, 100);

    // let ret = i2s_channel_write(
    //     i2s_tx_chan,
    //     c_void_ptr,
    //     (length * 3 * 2) as usize,
    //     &mut bytes_written,
    //     100,
    // );
    // if ret != ESP_OK {
    //     log::error!("i2s_channel_write failed");
    // }
}
