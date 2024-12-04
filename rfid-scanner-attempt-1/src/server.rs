use embedded_svc::http::Headers;
use esp_idf_hal::io::{Read, Write};
use esp_idf_svc::http::{server::EspHttpServer, Method};
use tokio::sync::mpsc::Sender;

use crate::common::SystemMessage;

static INDEX_HTML: &str = "Hello"; //include_str!("http_server_page.html");

// Max payload length
const MAX_LEN: usize = 128;

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

        let mut server = self.create_server()?;

        server.fn_handler("/", Method::Get, |req| {
            req.into_ok_response()?
                .write_all(INDEX_HTML.as_bytes())
                .map(|_| ())
        })?;

        server.fn_handler::<anyhow::Error, _>("/post", Method::Post, |mut req| {
            let len = req.content_len().unwrap_or(0) as usize;

            if len > MAX_LEN {
                req.into_status_response(413)?
                    .write_all("Request too big".as_bytes())?;
                return Ok(());
            }

            let mut buf = vec![0; len];
            req.read_exact(&mut buf)?;
            let mut resp = req.into_ok_response()?;

            // if let Ok(form) = serde_json::from_slice::<FormData>(&buf) {
            //     write!(
            //         resp,
            //         "Hello, {}-year-old {} from {}!",
            //         form.age, form.first_name, form.birthplace
            //     )?;
            // } else {
            resp.write_all("JSON error".as_bytes())?;
            // }

            Ok(())
        })?;

        // Keeps the server running in the background...
        core::mem::forget(server);

        Ok(())
    }
}
