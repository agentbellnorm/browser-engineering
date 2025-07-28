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
    let URL { path, host, .. } = url;

    let mut stream = TcpStream::connect(&format!("{host}:80")).unwrap();

    let mut request = String::new();

    request.push_str(&format!("GET {path} HTTP/1.0\r\n"));
    request.push_str(&format!("Host: {host}\r\n"));
    request.push_str(&format!("\r\n"));

    let _ = stream.write_all(request.as_bytes());

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response)?;

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
