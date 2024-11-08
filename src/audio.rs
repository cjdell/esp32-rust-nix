use crate::common;
use common::MAX_DELAY;
use esp_idf_sys::*;
use std::{
    ffi::{c_void, CString},
    ptr,
};

const BUFFER_SIZE_SAMPLES: usize = 16000;
const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

const DESIRED_CHUNK_SIZE_IN_BYTES: usize = 128;
const YOUR_DEFINED_THRESHOLD: f64 = 100f64;
const MIN_SILENCE_CHUNK_COUNT: usize = 1;

pub static mut RING_BUF: RingbufHandle_t = 0 as RingbufHandle_t;
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

    // fn write_to_i2s(samples: &[i16]) -> Result<(), esp_err_t> {
    //     let buffer = samples.as_ptr() as *const c_void;
    //     let item_size = samples.len() * std::mem::size_of::<i16>();
    //     let mut bytes_written: usize = 0;

    //     println!("Word");
    //     let ret = unsafe {
    //         i2s_channel_write(
    //             I2S_TX_CHAN,
    //             buffer,
    //             item_size,
    //             &mut bytes_written,
    //             MAX_DELAY,
    //         )
    //     };
    //     if ret != ESP_OK {
    //         log::error!("i2s_channel_write failed");
    //         return Err(ret);
    //     }
    //     if bytes_written != item_size {
    //         log::warn!("i2s_channel_write truncated");
    //     }
    //     Ok(())
    // }

    // extern "C" fn i2s_write_task(_parameter: *mut c_void) {
    //     let mut item_size: usize = 0;
    //     let max_chunk_size: usize = DESIRED_CHUNK_SIZE_IN_BYTES;
    //     let mut temp_buffer: Vec<i16> = Vec::new();
    //     const SILENCE_THRESHOLD: f64 = YOUR_DEFINED_THRESHOLD;
    //     let mut silence_counter: usize = 0;
    //     const SILENCE_CHUNKS: usize = MIN_SILENCE_CHUNK_COUNT; // Number of consecutive silent chunks to consider as silence

    //     loop {
    //         let buffer = unsafe {
    //             xRingbufferReceiveUpTo(RING_BUF, &mut item_size, MAX_DELAY, max_chunk_size)
    //         };

    //         print!(".");

    //         if buffer != ptr::null_mut() {
    //             // Process the buffer
    //             let samples =
    //                 unsafe { std::slice::from_raw_parts(buffer as *const i16, item_size / 2) };

    //             // Append samples to temp_buffer
    //             temp_buffer.extend_from_slice(samples);

    //             // Calculate the average power of the latest chunk
    //             let sum_squares: f64 = samples.iter().map(|&sample| (sample as f64).powi(2)).sum();
    //             let avg_power = (sum_squares / samples.len() as f64).sqrt();

    //             if avg_power < SILENCE_THRESHOLD {
    //                 silence_counter += 1;
    //             } else {
    //                 silence_counter = 0;
    //             }

    //             // If we've detected enough consecutive silent chunks, flush the buffer
    //             if silence_counter >= SILENCE_CHUNKS && !temp_buffer.is_empty() {
    //                 if let Err(err) = AudioService::write_to_i2s(&temp_buffer) {
    //                     // Handle the error
    //                 }
    //                 temp_buffer.clear();
    //                 silence_counter = 0;
    //             }

    //             // Return the buffer to the ring buffer
    //             unsafe { vRingbufferReturnItem(RING_BUF, buffer) };
    //         } else {
    //             // If no data is received, but temp_buffer is not empty, flush it
    //             // if !temp_buffer.is_empty() {
    //             //     if let Err(err) = AudioService::write_to_i2s(&temp_buffer) {
    //             //         // Handle the error
    //             //     }
    //             //     temp_buffer.clear();
    //             // }
    //             // Optional: Add a small delay to prevent tight looping
    //             // unsafe { vTaskDelay(1) };
    //         }
    //     }
    // }

    // pub unsafe fn write_samples_directly(buffer: *mut i16, sample_count: usize) {
    //     let mut bytes_written: usize = 0;

    //     let bytes_to_write = sample_count * std::mem::size_of::<i16>();

    //     let ret = i2s_channel_write(
    //         I2S_TX_CHAN,
    //         buffer as *mut c_void,
    //         bytes_to_write,
    //         &mut bytes_written,
    //         MAX_DELAY,
    //     );
    //     if ret != ESP_OK {
    //         log::error!("i2s_channel_write failed");
    //     }
    //     if bytes_written != bytes_to_write {
    //         log::warn!("i2s_channel_write truncated");
    //     }
    // }
}
