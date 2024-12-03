use crate::common;
use common::MAX_DELAY;
use esp_idf_sys::*;
use std::{
    f32::consts::PI,
    ffi::{c_void, CString},
};

const SAMPLE_RATE: usize = 16000;
const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

static mut I2S_TX_CHAN: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

#[derive(Copy, Clone)]
pub struct AudioService {}

impl AudioService {
    pub fn new() -> AudioService {
        let audio_service = AudioService {};

        audio_service.init();

        audio_service
    }

    fn init(&self) {
        let chan_cfg = i2s_chan_config_t {
            id: i2s_port_t_I2S_NUM_0,
            role: i2s_role_t_I2S_ROLE_MASTER,
            dma_desc_num: 6,
            dma_frame_num: 512,
            auto_clear: true,
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

        let tdm_tx_cfg = i2s_tdm_config_t {
            clk_cfg: i2s_tdm_clk_config_t {
                sample_rate_hz: SAMPLE_RATE,
                clk_src: soc_periph_i2s_clk_src_t_I2S_CLK_SRC_DEFAULT,
                mclk_multiple: i2s_mclk_multiple_t_I2S_MCLK_MULTIPLE_256,
                ext_clk_freq_hz: 0,
                bclk_div: 8,
            },
            slot_cfg: i2s_tdm_slot_config_t {
                big_endian: false,
                bit_order_lsb: false,
                bit_shift: true,
                data_bit_width: 16,
                left_align: false,
                skip_mask: false,
                slot_bit_width: 16,
                slot_mask: i2s_tdm_slot_mask_t_I2S_TDM_SLOT0,
                slot_mode: i2s_slot_mode_t_I2S_SLOT_MODE_MONO,
                total_slot: I2S_TDM_AUTO_SLOT_NUM,
                ws_pol: false,
                ws_width: I2S_TDM_AUTO_WS_WIDTH,
            },
            gpio_cfg: i2s_tdm_gpio_config_t {
                mclk: esp_idf_sys::I2S_PIN_NO_CHANGE,
                bclk: gpio_num_t_GPIO_NUM_6,
                din: esp_idf_sys::I2S_PIN_NO_CHANGE,
                dout: gpio_num_t_GPIO_NUM_44,
                invert_flags: i2s_tdm_gpio_config_t__bindgen_ty_1::default(),
                ws: gpio_num_t_GPIO_NUM_5,
            },
        };

        unsafe {
            let ret = i2s_channel_init_tdm_mode(I2S_TX_CHAN, &tdm_tx_cfg);
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
            let task_name = CString::new("I2SWriteTask").unwrap();

            xTaskCreatePinnedToCore(
                Some(AudioService::i2s_write_task),
                task_name.as_ptr(),
                2048,
                (0) as *mut c_void,
                I2S_PRI,
                (0) as *mut TaskHandle_t,
                I2S_CORE,
            )
        };
    }

    extern "C" fn i2s_write_task(_parameter: *mut c_void) {
        let mut count = 0;

        let buffer_size = SAMPLE_RATE / 10; // Buffer for 0.1 seconds of audio
        let mut buffer = vec![0i16; buffer_size];

        loop {
            const FREQUENCY: f32 = 440.0; // A4 frequency

            let freq = FREQUENCY * count as f32 / 10f32;

            // Generate 440 Hz sine wave in the buffer
            let amplitude = i16::MAX as f32 / 2.0; // Max amplitude for 16-bit signed integers
            for i in 0..buffer_size {
                let sample = amplitude * (2.0 * PI * freq * (i as f32) / SAMPLE_RATE as f32).sin();
                buffer[i] = sample as i16;
            }

            let c_buffer_size = buffer_size * std::mem::size_of::<i16>(); // Buffer size in bytes

            // Get raw pointer to buffer
            let c_buffer = buffer.as_ptr() as *const ::core::ffi::c_void;

            let mut bytes_written: usize = 0;

            // Write buffer to I2S channel
            let ret = unsafe {
                i2s_channel_write(
                    I2S_TX_CHAN,
                    c_buffer,
                    c_buffer_size,
                    &mut bytes_written,
                    MAX_DELAY,
                )
            };

            if ret != ESP_OK {
                log::error!("i2s_channel_write failed with error code {}", ret);
            }

            count += 1;

            if count > 10 {
                break;
            };
        }

        // Do nothing. Can't end task or watch dog will trigger...
        loop {
            unsafe { vTaskDelay(1000) };
        }
    }

    pub unsafe fn write_samples_directly(buffer: *mut i16, sample_count: usize) {
        let mut bytes_written: usize = 0;

        let bytes_to_write = sample_count * std::mem::size_of::<i16>();

        let ret = i2s_channel_write(
            I2S_TX_CHAN,
            buffer as *mut c_void,
            bytes_to_write,
            &mut bytes_written,
            MAX_DELAY,
        );
        if ret != ESP_OK {
            log::error!("i2s_channel_write failed");
        }
        if bytes_written != bytes_to_write {
            log::warn!("i2s_channel_write truncated");
        }
    }
}
