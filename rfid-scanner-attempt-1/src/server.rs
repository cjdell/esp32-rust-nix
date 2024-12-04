use std::sync::Arc;

use embedded_svc::http::Headers;
use esp_idf_hal::io::{Read, Write};
use esp_idf_svc::http::{server::EspHttpServer, Method};
use tokio::{runtime::Builder, sync::mpsc::Sender};

use crate::{common::SystemMessage, spiffs::Spiffs};

static INDEX_HTML: &str = "Hello"; //include_str!("http_server_page.html");

// Max payload length
const MAX_LEN: usize = 2048 * 1024;

// Need lots of stack to parse JSON
const STACK_SIZE: usize = 10240;

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

            Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { tx2.send(SystemMessage::OnOtaBuffer(Arc::new(buf))).await })
                .unwrap();

            resp.write_all(format!("Done {}", len).as_bytes())?;

            Ok(())
        })?;

        server.fn_handler("/read-file", Method::Get, |req| {
            let file_name: String = req.uri().split("?name=").nth(1).unwrap().to_string();

            let contents = Spiffs::read_string(file_name);

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

            Spiffs::write_string(file_name, String::from_utf8(buf).unwrap());

            resp.write_all(format!("Done {}", len).as_bytes())?;

            Ok(())
        })?;

        // Keeps the server running in the background...
        core::mem::forget(server);

        Ok(())
    }
}
