use crate::common;
use common::MAX_DELAY;
use esp_idf_sys::*;
use std::ffi::c_void;
use std::ffi::CString;

const BUFFER_SIZE_SAMPLES: usize = 2048;
const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

pub static mut RING_BUF: RingbufHandle_t = 0 as RingbufHandle_t;
static mut I2S_TX_CHAN: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

pub static mut RECV_CHUNKS: usize = 0;
pub static mut RECV_BYTES: usize = 0;

pub fn init_audio() {
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
