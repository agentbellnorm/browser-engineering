use rustls::{ClientConfig, ClientConnection, StreamOwned};
use std::io::ErrorKind;
use std::sync::Arc;
use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
    iter::Map,
    net::TcpStream,
    slice::SplitN,
};

use crate::url::URL;

#[derive(Debug)]
pub struct Response {
    pub status: u8,
    pub version: String,
    pub explanation: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
}

pub fn get(url: URL) -> Result<Response, Box<dyn Error>> {
    let response = match url.scheme.as_str() {
        "http" => get_http_response(url),
        "https" => get_https_response(url),
        _ => Err("unsupported scheme".into()),
    }?;

    println!("BEGIN RESPONSE:");
    println!("{response}");
    println!("END RESPONSE:");

    let mut response_lines = response.split("\r\n");

    let (version, status, explanation) = split_response_status_line(response_lines.next())?;

    let mut headers: HashMap<String, String> = HashMap::new();

    loop {
        if let Some(current_line) = response_lines.next() {
            if let Some((key, value)) = current_line.split_once(":") {
                headers.insert(key.trim().to_string(), value.trim().to_string());
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if headers.contains_key("transfer-encoding") || headers.contains_key("content-encoding") {
        return Err("body is encoded, dont know how to handle that".into());
    }

    Ok(Response {
        status,
        explanation: explanation.to_string(),
        version: version.to_string(),
        body: response_lines.next().map(|rest| rest.trim().to_string()),
        headers,
    })
}

fn get_http_response(url: URL) -> Result<String, Box<dyn Error>> {
    let URL {
        path, host, port, ..
    } = url;

    let port_to_use = port.unwrap_or("80".to_string());

    let mut stream = TcpStream::connect(&format!("{host}:{port_to_use}")).unwrap();

    let mut request = String::new();

    request.push_str(&format!("GET {path} HTTP/1.0\r\n"));
    request.push_str(&format!("Host: {host}\r\n"));
    request.push_str(&format!("\r\n"));

    let _ = stream.write_all(request.as_bytes());

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response)?;

    Ok(response)
}

fn get_https_response(url: URL) -> Result<String, Box<dyn Error>> {
    let URL {
        path, host, port, ..
    } = url;

    let port_to_use = port.unwrap_or("443".to_string());
    let domain = format!("{host}:{port_to_use}");

    let mut request = String::new();
    request.push_str(&format!("GET {path} HTTP/1.0\r\n"));
    request.push_str(&format!("Host: {host}\r\n"));
    request.push_str(&format!("\r\n"));

    // Step 1: Prepare root certificates
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Step 2: Build TLS client config
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let config = Arc::new(config);
    // Step 3: Connect TCP stream
    let tcp_stream = TcpStream::connect(domain.clone()).map_err(|e| {
        println!("could not create tcp connection: {e}");
        e
    })?;

    // Step 4: Create TLS connection
    let server_name = host.try_into()?;
    let tls_conn = ClientConnection::new(config, server_name).map_err(|e| {
        println!("could not create tls connection: {e}");
        e
    })?;

    let mut tls_stream = StreamOwned::new(tls_conn, tcp_stream);

    tls_stream.write_all(request.as_bytes()).map_err(|e| {
        println!("could not write to stream: {e}");
        e
    })?;

    let mut response = Vec::new();

    match tls_stream.read_to_end(&mut response) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            // harmless - the server closed without close_notify
        }
        Err(e) => return Err(e.into()),
    }

    Ok(String::from_utf8_lossy(&response).to_string())
}

fn split_response_status_line(
    _status_line: Option<&str>,
) -> Result<(&str, u8, &str), Box<dyn Error>> {
    if let Some(status_line) = _status_line {
        let mut line_iter = status_line.splitn(3, " ");
        let (_version, _status, _explanation) =
            (line_iter.next(), line_iter.next(), line_iter.next());

        return match (_version, _status, _explanation) {
            (Some(version), Some(status), Some(explanation)) => {
                Ok((version, status.parse()?, explanation))
            }
            _ => Err(format!("status line could not be split in three {status_line}").into()),
        };
    }

    Err("status line was None".into())
}
