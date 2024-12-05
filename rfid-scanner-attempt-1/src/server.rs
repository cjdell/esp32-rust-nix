use crate::{
    audio,
    common::{self, SystemMessage},
    spiffs::Spiffs,
};
use embedded_svc::http::Headers;
use esp_idf_hal::io::{Read, Write};
use esp_idf_svc::http::{server::EspHttpServer, Method};
use esp_idf_sys::xRingbufferSend;
use std::{os::raw::c_void, sync::Arc};
use tokio::{runtime::Builder, sync::mpsc::Sender};

static INDEX_HTML: &str = "Hello"; //include_str!("http_server_page.html");

// Max payload length
const MAX_LEN: usize = 2048 * 1024;

// Need lots of stack to parse JSON
const STACK_SIZE: usize = 10240;

macro_rules! call_async {
    ($async_code:block) => {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { $async_code })
            .unwrap();
    };
}

pub struct HttpServer {
    tx: Sender<SystemMessage>,
    started: bool,
}

impl HttpServer {
    pub fn new(tx: Sender<SystemMessage>) -> HttpServer {
        HttpServer { tx, started: false }
    }

    fn create_server(&self) -> anyhow::Result<EspHttpServer<'static>> {
        let server_configuration = esp_idf_svc::http::server::Configuration {
            stack_size: STACK_SIZE,
            http_port: 80,
            ..Default::default()
        };

        Ok(EspHttpServer::new(&server_configuration)?)
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.started {
            return Ok(());
        }

        self.started = true;

        let tx1 = self.tx.clone();
        let tx2 = self.tx.clone();
        let tx3 = self.tx.clone();
        let tx4 = self.tx.clone();

        let mut server = self.create_server()?;

        server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(INDEX_HTML.as_bytes())
                .map(|_| ())
        })?;

        server.fn_handler("/say", Method::Get, move |req| {
            let msg: String = req.uri().split("?msg=").nth(1).unwrap().to_string();

            Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { tx1.send(SystemMessage::Speak(msg)).await })
                .unwrap();

            req.into_ok_response()?
                .write_all("OK".as_bytes())
                .map(|_| ())
        })?;

        server.fn_handler::<anyhow::Error, _>("/update", Method::Post, move |mut req| {
            let len = req.content_len().unwrap_or(0) as usize;

            if len > MAX_LEN {
                req.into_status_response(413)?
                    .write_all("Request too big".as_bytes())?;
                return Ok(());
            }

            let mut buf = vec![0; len];
            req.read_exact(&mut buf)?;

            let mut resp = req.into_ok_response()?;

            call_async!({ tx2.send(SystemMessage::OnOtaBuffer(Arc::new(buf))).await });

            resp.write_all(format!("Done {}", len).as_bytes())?;

            Ok(())
        })?;

        server.fn_handler("/read-file", Method::Get, |req| {
            let file_name: String = req.uri().split("?name=").nth(1).unwrap().to_string();

            let contents = Spiffs::read_string(file_name).unwrap();

            req.into_ok_response()?
                .write_all(contents.as_bytes())
                .map(|_| ())
        })?;

        server.fn_handler::<anyhow::Error, _>("/write-file", Method::Post, move |mut req| {
            let file_name: String = req.uri().split("?name=").nth(1).unwrap().to_string();

            let len = req.content_len().unwrap_or(0) as usize;

            if len > MAX_LEN {
                req.into_status_response(413)?
                    .write_all("Request too big".as_bytes())?;
                return Ok(());
            }

            let mut buf = vec![0; len];
            req.read_exact(&mut buf)?;

            let mut resp = req.into_ok_response()?;

            Spiffs::write_binary(file_name, buf);

            call_async!({
                tx4.send(SystemMessage::Speak("File written.".to_string()))
                    .await
            });

            resp.write_all(format!("Done {}", len).as_bytes())?;

            Ok(())
        })?;

        // ffmpeg -i denybeep2.mp3 -ar 16000 -ac 1 -sample_fmt s16 spiffs/denied.wav

        server.fn_handler("/play", Method::Get, move |req| {
            let file_name: String = req.uri().split("?name=").nth(1).unwrap().to_string();

            let contents = Spiffs::read_binary(file_name).unwrap();

            let c_buffer = contents.as_ptr() as *mut c_void;

            unsafe {
                xRingbufferSend(audio::RING_BUF, c_buffer, contents.len(), common::MAX_DELAY)
            };

            req.into_ok_response()?
                .write_all("OK".as_bytes())
                .map(|_| ())
        })?;

        // Keeps the server running in the background...
        core::mem::forget(server);

        Ok(())
    }
}
