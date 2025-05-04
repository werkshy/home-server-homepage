use std::{
    error::Error,
    fs::{self},
};

use serde::Deserialize;
use tiny_http::{Response, Server};

const DEFAULT_PORT: u32 = 8000;
const CONFIG_FILE: &str = "config.toml";

#[derive(Deserialize, PartialEq, Debug)]
struct Link {
    title: String,
    port: Option<u32>,
    url: Option<String>,
    sub_heading: Option<String>,
}

#[derive(Deserialize, PartialEq, Debug, Default)]
struct ServerConfig {
    port: Option<u32>,
}

#[derive(Deserialize, PartialEq, Debug)]
struct Config {
    server: Option<ServerConfig>,
    link: Vec<Link>,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let config = get_config(CONFIG_FILE)?;
    let port = config
        .server
        .unwrap_or_default()
        .port
        .unwrap_or(DEFAULT_PORT);
    println!("Starting server on port {port} - http://localhost:{port}");
    let server = Server::http(format!("0.0.0.0:{port}"))?;

    let template = include_str!("templates/index.html");
    let section_html = include_str!("templates/section.html");
    let favicon = include_bytes!("templates/favicon.png");

    let content_type_html =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap();
    let content_type_png =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap();

    for request in server.incoming_requests() {
        // We reload the config in each request because it's fast enough and it makes editing
        // and testing config easier.
        let config = get_config(CONFIG_FILE)?;
        println!("{:?} {:?}", request.method(), request.url());

        // Handle favicon requests
        if request.url() == "/favicon.png" {
            let response = Response::from_data(favicon).with_header(content_type_png.clone());
            let _ = request.respond(response);
            continue;
        }

        // Render the page
        let hostname = match request.headers().iter().find(|h| h.field.equiv("Host")) {
            Some(header) => &header.value,
            None => {
                let response = Response::from_string("No Host header").with_status_code(400);
                let _ = request.respond(response);
                continue;
            }
        };

        let sections = config
            .link
            .iter()
            .map(|link| render_section(section_html, hostname.as_str(), link).unwrap())
            .collect::<Vec<String>>();
        let body = template.replace("__SECTIONS__", &sections.join("\n"));

        let response = Response::from_string(body).with_header(content_type_html.clone());
        request.respond(response)?;
    }
    Ok(())
}

fn get_config(config_file: &str) -> Result<Config, String> {
    let toml_data = fs::read_to_string(config_file).map_err(|err| err.to_string())?;
    let config: Config = toml::from_str(toml_data.as_str()).map_err(|err| err.to_string())?;
    Ok(config)
}

fn render_section(template: &str, host: &str, link: &Link) -> Result<String, String> {
    let url = match (link.port, &link.url) {
        (_, Some(url)) => url.clone(),
        (Some(port), None) => set_port(host, port),
        (None, None) => String::from("/"),
    };

    let sub_heading = link.sub_heading.as_deref().unwrap_or("/");

    Ok(template
        .replace("__URL__", &url)
        .replace("__TITLE__", &link.title)
        .replace("__SUB_HEADING__", sub_heading))
}

fn set_port(host: &str, port: u32) -> String {
    let mut parts = host.splitn(2, ':');
    let hostname = parts.next().unwrap_or(host);
    format!("//{hostname}:{port}")
}
