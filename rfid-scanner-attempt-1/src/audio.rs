use crate::common;
use common::MAX_DELAY;
use esp_idf_sys::*;
use std::ffi::{c_void, CString};

const BUFFER_SIZE_SAMPLES: usize = 16000;
const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

pub static mut RING_BUF: RingbufHandle_t = 0 as RingbufHandle_t;
static mut I2S_TX_CHAN: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

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
            dma_frame_num: 240,
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
                sample_rate_hz: 16000,
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
                bclk: gpio_num_t_GPIO_NUM_2,
                din: esp_idf_sys::I2S_PIN_NO_CHANGE,
                dout: gpio_num_t_GPIO_NUM_1,
                invert_flags: i2s_tdm_gpio_config_t__bindgen_ty_1::default(),
                ws: gpio_num_t_GPIO_NUM_3,
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
            RING_BUF = xRingbufferCreate(
                BUFFER_SIZE_SAMPLES * std::mem::size_of::<i16>(),
                RingbufferType_t_RINGBUF_TYPE_BYTEBUF,
            );
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
        let mut item_size: usize = 0;

        loop {
            // This is a `*mut c_void` that contains `i16` mono samples
            let buffer = unsafe { xRingbufferReceive(RING_BUF, &mut item_size, MAX_DELAY) };

            if buffer != 0 as *mut c_void {
                let mut bytes_written: usize = 0;

                // RECV_CHUNKS += 1;
                // RECV_BYTES += item_size;

                // print!("O");
                let ret = unsafe {
                    i2s_channel_write(
                        I2S_TX_CHAN,
                        buffer,
                        item_size,
                        &mut bytes_written,
                        MAX_DELAY,
                    )
                };
                if ret != ESP_OK {
                    log::error!("i2s_channel_write failed");
                }
                if bytes_written != item_size {
                    log::warn!("i2s_channel_write truncated");
                }

                unsafe { vRingbufferReturnItem(RING_BUF, buffer) };

                unsafe { vTaskDelay(10) };
            }
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
