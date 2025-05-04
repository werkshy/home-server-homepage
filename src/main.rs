use std::fs::{self};

use serde::Deserialize;
use tiny_http::{Response, Server};

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

fn main() {
    let config = get_config("config.toml").unwrap();
    let port = config.server.unwrap_or_default().port.unwrap_or(8000);
    println!("Starting server on port {port}");
    let server = Server::http(format!("0.0.0.0:{port}")).unwrap();
    let content_type_header =
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap();

    let template = include_str!("templates/index.html");
    let section_html = include_str!("templates/section.html");
    let favicon = include_bytes!("templates/favicon.png");

    for request in server.incoming_requests() {
        println!("{:?} {:?}", request.method(), request.url());

        // Handle favicon requests
        if request.url() == "/favicon.png" {
            let response = Response::from_data(favicon).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"image/png"[..]).unwrap(),
            );
            let _ = request.respond(response);
            continue;
        }

        // Render the page
        let hostname_header = request.headers().iter().find(|h| h.field.equiv(&"Host"));
        if hostname_header.is_none() {
            let response = Response::from_string("No Host header").with_status_code(400);
            let _ = request.respond(response);
            continue;
        }
        let hostname = hostname_header.unwrap().value.clone();
        println!("Header: {:?}", hostname);

        let sections = config
            .link
            .iter()
            .map(|link| render_section(section_html, hostname.as_str(), link).unwrap())
            .collect::<Vec<String>>();
        let body = template.replace("__SECTIONS__", &sections.join("\n"));

        let response = Response::from_string(body).with_header(content_type_header.clone());
        request.respond(response).unwrap();
    }
}

fn get_config(config_file: &str) -> Result<Config, String> {
    let toml_data = fs::read_to_string(config_file).map_err(|err| err.to_string())?;
    let config: Config = toml::from_str(toml_data.as_str()).map_err(|err| err.to_string())?;
    return Ok(config);
}

fn render_section(template: &str, host: &str, link: &Link) -> Result<String, String> {
    // TODO handle port vs url
    let mut url = String::from("/");
    if link.port.is_some() {
        url = set_port(host, link.port.unwrap());
    }
    if link.url.is_some() {
        url = link.url.clone().unwrap();
    }
    let sub_heading = link.sub_heading.clone().unwrap_or(String::from("/"));

    return Ok(template
        .replace("__URL__", url.as_str())
        .replace("__TITLE__", link.title.as_str())
        .replace("__SUB_HEADING__", sub_heading.as_str()));
}

fn set_port(host: &str, port: u32) -> String {
    if host.contains(":") {
        let hostname = host.split_once(":").unwrap().0;
        return format!("//{}:{}", hostname, port);
    }
    return format!("//{}:{}", host, port);
}
