use crate::common;
use common::MAX_DELAY;
use esp_idf_hal::{
    delay::TickType,
    gpio,
    i2s::{
        self,
        config::{
            Config, DataBitWidth, SlotMode, StdClkConfig, StdConfig, StdGpioConfig, StdSlotConfig,
            TdmClkConfig, TdmConfig, TdmGpioConfig, TdmSlotConfig, TdmSlotMask,
        },
        I2sDriver, I2sTx,
    },
    prelude::Peripherals,
};
use esp_idf_sys::*;
use lazy_static::lazy_static;
use std::{
    ffi::{c_void, CString},
    ptr::null,
    sync::{Arc, Mutex},
};

const BUFFER_SIZE_SAMPLES: usize = 16000;
const I2S_CORE: i32 = 1;
const I2S_PRI: u32 = 22;

const DESIRED_CHUNK_SIZE_IN_BYTES: usize = 128;
const YOUR_DEFINED_THRESHOLD: f64 = 100f64;
const MIN_SILENCE_CHUNK_COUNT: usize = 1;

pub static mut RING_BUF: RingbufHandle_t = 0 as RingbufHandle_t;
static mut I2S_TX_CHAN: i2s_chan_handle_t = (0 as *mut i2s_channel_obj_t) as i2s_chan_handle_t;

// lazy_static! {
//     static ref I2S_DRIVER: Arc<Mutex<Option<I2sDriver<'_, I2sTx>>>> = Arc::new(Mutex::new(None));
// }

static mut I2S_DRIVER: *mut I2sDriver<'_, I2sTx> = 0 as *mut I2sDriver<'_, I2sTx>;

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

        // let pdm_tx_cfg = i2s_pdm_tx_config_t {
        //     clk_cfg: i2s_pdm_tx_clk_config_t {
        //         sample_rate_hz: 16000,
        //         clk_src: soc_periph_i2s_clk_src_t_I2S_CLK_SRC_DEFAULT,
        //         mclk_multiple: i2s_mclk_multiple_t_I2S_MCLK_MULTIPLE_256,
        //         up_sample_fp: 960,
        //         up_sample_fs: 480,
        //         bclk_div: 8,
        //     },
        //     slot_cfg: i2s_pdm_tx_slot_config_t {
        //         data_bit_width: i2s_data_bit_width_t_I2S_DATA_BIT_WIDTH_16BIT,
        //         slot_bit_width: i2s_slot_bit_width_t_I2S_SLOT_BIT_WIDTH_16BIT,
        //         slot_mode: i2s_slot_mode_t_I2S_SLOT_MODE_MONO,
        //         sd_prescale: 0,
        //         sd_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
        //         hp_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_DIV_2,
        //         lp_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
        //         sinc_scale: i2s_pdm_sig_scale_t_I2S_PDM_SIG_SCALING_MUL_1,
        //         line_mode: i2s_pdm_tx_line_mode_t_I2S_PDM_TX_ONE_LINE_DAC,
        //         hp_en: true,
        //         hp_cut_off_freq_hz: 35.5,
        //         sd_dither: 0,
        //         sd_dither2: 1,
        //     },
        //     gpio_cfg: i2s_pdm_tx_gpio_config_t {
        //         clk: gpio_num_t_GPIO_NUM_6,
        //         dout: gpio_num_t_GPIO_NUM_44,
        //         dout2: 0,
        //         invert_flags,
        //     },
        // };

        // unsafe {
        //     let ret = i2s_channel_init_pdm_tx_mode(I2S_TX_CHAN, &pdm_tx_cfg);
        //     if ret != ESP_OK {
        //         log::error!("i2s_channel_init_pdm_tx_mode failed");
        //     }
        // }

        // unsafe {
        //     let ret = i2s_channel_enable(I2S_TX_CHAN);
        //     if ret != ESP_OK {
        //         log::error!("i2s_channel_enable failed");
        //     }
        // }

        // unsafe {
        //     RING_BUF = xRingbufferCreate(
        //         BUFFER_SIZE_SAMPLES * std::mem::size_of::<i16>(),
        //         RingbufferType_t_RINGBUF_TYPE_BYTEBUF,
        //     );
        // }

        unsafe {
            let task_name = CString::new("I2SWriteTask").unwrap();

            // xTaskCreatePinnedToCore(
            //     Some(AudioService::i2s_write_task),
            //     task_name.as_ptr(),
            //     2048,
            //     (0) as *mut c_void,
            //     I2S_PRI,
            //     (0) as *mut TaskHandle_t,
            //     I2S_CORE,
            // )
        };

        const I2S_PORT_NUM: u32 = 0;
        const SAMPLE_RATE: u32 = 16000;
        const CHANNEL_COUNT: u16 = 1;

        // let config = esp_idf_sys::i2s_driver_config_t {
        //     mode: esp_idf_sys::i2s_mode_t_I2S_MODE_MASTER | esp_idf_sys::i2s_mode_t_I2S_MODE_TX,
        //     sample_rate: SAMPLE_RATE,
        //     bits_per_sample: esp_idf_sys::i2s_bits_per_chan_t_I2S_BITS_PER_CHAN_16BIT,
        //     channel_format: esp_idf_sys::i2s_channel_fmt_t_I2S_CHANNEL_FMT_ONLY_RIGHT,
        //     communication_format: esp_idf_sys::i2s_comm_format_t_I2S_COMM_FORMAT_STAND_I2S,
        //     intr_alloc_flags: esp_idf_sys::ESP_INTR_FLAG_LEVEL1 as i32, // Interrupt level 1, default 0
        //     // dma_buf_count: 8,
        //     // dma_buf_len: 64,
        //     use_apll: false,
        //     tx_desc_auto_clear: false,
        //     fixed_mclk: 0,
        //     mclk_multiple: esp_idf_sys::i2s_mclk_multiple_t_I2S_MCLK_MULTIPLE_256,
        //     bits_per_chan: 0,
        //     bit_order_msb: false,
        //     big_edin: false,
        //     left_align: false,
        //     chan_mask: esp_idf_sys::i2s_channel_t_I2S_CHANNEL_MONO,
        //     total_chan: 0,
        //     skip_msk: false,
        //     __bindgen_anon_1: i2s_driver_config_t__bindgen_ty_1 { dma_buf_count: 8 },
        //     __bindgen_anon_2: i2s_driver_config_t__bindgen_ty_2 { dma_buf_len: 64 },
        // };

        // let result = unsafe {
        //     esp_idf_sys::i2s_driver_install(I2S_PORT_NUM, &config, 0, std::ptr::null_mut())
        // };
        // if result != esp_idf_sys::ESP_OK {
        //     panic!("error installing i2s driver");
        // }

        // let pin_config = esp_idf_sys::i2s_pin_config_t {
        //     mck_io_num: esp_idf_sys::I2S_PIN_NO_CHANGE, // unused
        //     bck_io_num: esp_idf_sys::gpio_num_t_GPIO_NUM_6,
        //     ws_io_num: esp_idf_sys::gpio_num_t_GPIO_NUM_5, // LR clock
        //     data_out_num: esp_idf_sys::gpio_num_t_GPIO_NUM_44,
        //     data_in_num: esp_idf_sys::I2S_PIN_NO_CHANGE,
        // };

        // let result = unsafe { esp_idf_sys::i2s_set_pin(I2S_PORT_NUM, &pin_config) };
        // if result != esp_idf_sys::ESP_OK {
        //     panic!("error setting i2s pins");
        // }

        // let i2s_config = StdConfig::new(
        //     Config::default(),
        //     StdClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        //     StdSlotConfig::philips_slot_default(DataBitWidth::Bits16, SlotMode::Mono),
        //     StdGpioConfig::default(),
        // );

        // let i2s_config2 = TdmConfig::new(
        //     Config::default(),
        //     TdmClkConfig::from_sample_rate_hz(SAMPLE_RATE),
        //     TdmSlotConfig::pcm_long_slot_default(
        //         DataBitWidth::Bits16,
        //         TdmSlotMask::from_mask_value(i2s_tdm_slot_mask_t_I2S_TDM_SLOT0 as u16),
        //     ),
        //     TdmGpioConfig::default(),
        // );

        // let peripherals = Peripherals::take().unwrap();

        // let i2s = peripherals.i2s0;

        // let bclk = peripherals.pins.gpio6;
        // let dout = peripherals.pins.gpio44;
        // let mclk: Option<gpio::AnyIOPin> = None;
        // let ws = peripherals.pins.gpio5;

        // // let mut driver = I2sDriver::new_std_tx(i2s, &i2s_config, bclk, dout, mclk, ws).unwrap();
        // let mut driver2 = I2sDriver::new_tdm_tx(i2s, &i2s_config2, bclk, dout, mclk, ws).unwrap();

        // driver2.tx_enable().unwrap();

        // unsafe {
        //     I2S_DRIVER = &mut driver2;
        // };
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

                // unsafe { vTaskDelay(10) };
            }
        }
    }

    pub unsafe fn write_samples_directly(buffer: *mut i16, sample_count: usize) {
        let mut bytes_written: usize = 0;

        let bytes_to_write = sample_count * std::mem::size_of::<i16>();

        // if I2S_DRIVER.is_null() {
        //     return;
        // };

        // const BLOCK_TIME: TickType = TickType::new(100_000_000);

        // let byte_slice =
        //     unsafe { core::slice::from_raw_parts(buffer as *const u8, sample_count * 2) };

        // let mut i2s_driver = I2S_DRIVER.read();

        // i2s_driver.write_all(byte_slice, BLOCK_TIME.into()).unwrap();

        // let ret = i2s_write(
        //     0,
        //     buffer as *mut c_void,
        //     bytes_to_write,
        //     &mut bytes_written,
        //     MAX_DELAY,
        // );
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
