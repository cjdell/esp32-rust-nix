use anyhow::bail;
use core::str;
use embedded_svc::http::client::Client;
use esp_idf_hal::io::Read;
use esp_idf_svc::http::client::EspHttpConnection;
use tokio::sync::mpsc::Sender;

use crate::{common::SystemMessage, spiffs::Spiffs};

pub struct AuthService {
    tx: Sender<SystemMessage>,
}

impl AuthService {
    pub fn new(tx: Sender<SystemMessage>) -> AuthService {
        AuthService { tx }
    }

    pub async fn check_text(&self, code: u32) -> anyhow::Result<bool> {
        let codes =
            Spiffs::read_string("codes.txt".to_string()).unwrap_or_else(|err| "".to_string());

        let lines = codes.split_whitespace();

        for line in lines {
            if code.to_string() == line {
                self.tx
                    .send(SystemMessage::OnAuth(code, code.to_string(), true))
                    .await?;

                return Ok(true);
            };
        }

        self.tx
            .send(SystemMessage::OnAuth(code, "".to_string(), false))
            .await?;

        Ok(false)
    }

    pub async fn check_server(&self, code: u32) -> anyhow::Result<()> {
        // HTTP Configuration
        // Create HTTPS Connection Handle
        let httpconnection = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration {
            use_global_ca_store: false,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;

        // Create HTTPS Client
        let mut httpclient = Client::wrap(httpconnection);

        // HTTP Request Submission
        // Define URL
        let url = "http://10.3.2.151:8000";

        // Prepare request
        let request = httpclient.get(url)?;

        // Log URL and type of request
        println!("-> GET {}", url);

        // Submit Request and Store Response
        let response = request.submit()?;

        // HTTP Response Processing
        let status = response.status();
        println!("<- {}", status);

        match response.header("Content-Length") {
            Some(data) => {
                println!("Content-Length: {}", data);
            }
            None => {
                println!("No Content-Length Header");
            }
        }

        match response.header("Date") {
            Some(data) => {
                println!("Date: {}", data);
            }
            None => {
                println!("No Date Header");
            }
        }

        match status {
            200..=299 => {
                // 5. If the status is OK, read response data chunk by chunk into a buffer and print it until done.
                //
                // NB. There is no guarantee that chunks will be split at the boundaries of valid UTF-8
                // sequences (in fact it is likely that they are not) so this edge case needs to be handled.
                // However, for the purposes of clarity and brevity(?), the additional case of completely invalid
                // UTF-8 sequences will not be handled here and is left as an exercise for the reader.
                let mut buf = [0_u8; 256];
                // Offset into the buffer to indicate that there may still be
                // bytes at the beginning that have not been decoded yet
                let mut offset = 0;
                // Keep track of the total number of bytes read to print later
                let mut total = 0;
                let mut reader = response;

                loop {
                    // read into the buffer starting at the offset to not overwrite
                    // the incomplete UTF-8 sequence we put there earlier
                    if let Ok(size) = Read::read(&mut reader, &mut buf[offset..]) {
                        if size == 0 {
                            // It might be nice to check if we have any left over bytes here (ie. the offset > 0)
                            // as this would mean that the response ended with an invalid UTF-8 sequence, but for the
                            // purposes of this training we are assuming that the full response will be valid UTF-8
                            break;
                        }
                        // Update the total number of bytes read
                        total += size;
                        // 6. Try converting the bytes into a Rust (UTF-8) string and print it.
                        // Remember that we read into an offset and recalculate the real length
                        // of the bytes to decode.
                        let size_plus_offset = size + offset;
                        match str::from_utf8(&buf[..size_plus_offset]) {
                            Ok(text) => {
                                // buffer contains fully valid UTF-8 data,
                                // print it and reset the offset to 0.
                                print!("{}", text);
                                offset = 0;
                            }
                            Err(error) => {
                                // The buffer contains incomplete UTF-8 data, we will
                                // print the valid part, copy the invalid sequence to
                                // the beginning of the buffer and set an offset for the
                                // next read.
                                //
                                // NB. There is actually an additional case here that should be
                                // handled in a real implementation. The Utf8Error may also contain
                                // an error_len field indicating that there is actually an invalid UTF-8
                                // sequence in the middle of the buffer. Such an error would not be
                                // recoverable through our offset and copy mechanism. The result will be
                                // that the invalid sequence will be copied to the front of the buffer and
                                // eventually the buffer will be filled until no more bytes can be read when
                                // the offset == buf.len(). At this point the loop will exit without reading
                                // any more of the response.
                                let valid_up_to = error.valid_up_to();
                                unsafe {
                                    // It's ok to use unsafe here as the error code already told us that
                                    // the UTF-8 data up to this point is valid, so we can tell the compiler
                                    // it's fine.
                                    print!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                                }
                                buf.copy_within(valid_up_to.., 0);
                                offset = size_plus_offset - valid_up_to;
                            }
                        }
                    }
                }

                println!("\nTotal: {} bytes", total);

                self.tx
                    .send(SystemMessage::OnAuth(code, "Insert name here".to_string(), true))
                    .await?;
            }
            _ => bail!("Unexpected response code: {}", status),
        }

        Ok(())
    }
}
