use crate::{
    audio::{self, AudioService},
    common,
};
use esp_idf_sys::{picotts_add, picotts_init, vTaskDelay, xRingbufferSend};
use std::ffi::{c_void, CString};

const TTS_CORE: i32 = 1;
const TTS_PRI: u32 = 20;

static mut SENT_CHUNKS: usize = 0;
static mut SENT_BYTES: usize = 0;

#[derive(Copy, Clone)]
pub struct SpeechService {
    audio_service: AudioService,
}

impl SpeechService {
    pub fn new(audio_service: AudioService) -> SpeechService {
        unsafe {
            picotts_init(TTS_PRI, Some(SpeechService::on_samples), TTS_CORE);
        }

        SpeechService { audio_service }
    }

    pub fn speak(&self, str: String) {
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

    unsafe extern "C" fn on_samples(buffer: *mut i16, length: u32) {
        // let factor = 3;
        let length = length as usize;

        // AudioService::write_samples_directly(buffer, length);

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
        xRingbufferSend(audio::RING_BUF, c_buffer, bytes, common::MAX_DELAY);

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
}
