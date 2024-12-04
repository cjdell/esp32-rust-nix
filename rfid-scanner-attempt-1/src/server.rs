use std::sync::Arc;

use embedded_svc::http::Headers;
use esp_idf_hal::io::{Read, Write};
use esp_idf_svc::{
    http::{
        client::EspHttpConnection,
        server::{EspHttpServer, Request},
        Method,
    },
    ota::EspOta,
};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::Sender,
};

use crate::common::SystemMessage;

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

        let tx = self.tx.clone();
        let tx2 = self.tx.clone();

        let mut server = self.create_server()?;

        server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(INDEX_HTML.as_bytes())
                .map(|_| ())
        })?;

        server.fn_handler("/say", Method::Get, move |req| {
            let msg: String = req.uri().split("?msg=").nth(1).unwrap().to_string();

            let tx = tx.clone();

            Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async { tx.send(SystemMessage::Speak(msg)).await })
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

        // Keeps the server running in the background...
        core::mem::forget(server);

        Ok(())
    }

    fn ota(data: &[u8]) {
        let mut ota = EspOta::new().expect("obtain OTA instance");

        let mut update = ota.initiate_update().expect("initiate OTA");

        update.write(&data).expect("write OTA data");

        update.complete().expect("complete OTA");

        esp_idf_svc::hal::reset::restart();
    }
}
