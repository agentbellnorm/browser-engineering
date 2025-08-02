use rustls::{ClientConnection, StreamOwned};
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use std::{
    collections::HashMap,
    error::Error,
    io::{Read, Write},
    net::TcpStream,
};

use crate::url::URL;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub version: String,
    pub explanation: String,
    pub body: Option<String>,
    pub headers: HashMap<String, String>,
    pub request: Request,
}

type Headers = HashMap<String, String>;

#[derive(Debug, Clone)]
enum Method {
    GET,
    POST,
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
struct Request {
    url: URL,
    headers: Option<Headers>,
    method: Method,
    body: Option<String>,
}

pub fn post(
    url: URL,
    body: Option<String>,
    headers: Option<Headers>,
) -> Result<Response, Box<dyn Error>> {
    todo!()
}

pub fn get(url: String, headers: Option<Headers>) -> Result<Response, Box<dyn Error>> {
    let request = Request {
        method: Method::GET,
        url: URL::parse(&url)?,
        headers,
        body: None,
    };

    do_request(request, 0)
}

fn do_request(request: Request, redirect_count: u8) -> Result<Response, Box<dyn Error>> {

    println!("Doing request {:?}", request);

    if redirect_count > 20 {
        return Err("Too many redirects".into());
    }

    let raw_request = raw_http_request(&request);

    println!("BEGIN REQUEST");
    println!("{raw_request}");
    println!("END REQUEST");

    let response = match request.url.scheme.as_str() {
        "http" => do_http_request(&raw_request, &request.url),
        "https" => do_https_request(&raw_request, &request.url),
        _ => Err("unsupported scheme".into()),
    }?;

    println!("BEGIN RESPONSE:");
    println!("{response}");
    println!("END RESPONSE:");

    let parsed = parse_response(response, request)?;

    match parsed.status {
        100..200 => Ok(parsed),
        200..300 => Ok(parsed),
        300..400 => do_request(get_next_request(&parsed)?, redirect_count + 1),
        400..500 => todo!("your fault"),
        500..600 => todo!("my fault"),
        _ => Err("unknown http code".into()),
    }
}

fn parse_response(response: String, request: Request) -> Result<Response, Box<dyn Error>> {
    let mut response_lines = response.split("\r\n");

    let (version, status, explanation) = split_response_status_line(response_lines.next())?;

    let mut headers: HashMap<String, String> = HashMap::new();

    loop {
        if let Some(current_line) = response_lines.next() {
            if let Some((key, value)) = current_line.split_once(":") {
                headers.insert(
                    key.trim().to_lowercase().to_string(),
                    value.trim().to_string(),
                );
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
        request,
    })
}

fn do_http_request(request: &str, url: &URL) -> Result<String, Box<dyn Error>> {
    let domain = url.domain();
    let mut stream = TcpStream::connect(domain).unwrap();

    let _ = stream.write_all(request.as_bytes());

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response)?;

    Ok(response)
}

fn do_https_request(request: &str, url: &URL) -> Result<String, Box<dyn Error>> {
    //
    // Step 1: Prepare root certificates
    let root_store =
        rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Step 2: Build TLS client config
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let config = Arc::new(config);
    // Step 3: Connect TCP stream
    let tcp_stream = TcpStream::connect(url.domain()).map_err(|e| {
        println!("could not create tcp connection: {e}");
        e
    })?;

    println!("tcp connected");

    // Step 4: Create TLS connection
    let server_name = url.host.clone().try_into()?;
    let tls_conn = ClientConnection::new(config, server_name).map_err(|e| {
        println!("could not create tls connection: {e}");
        e
    })?;

    println!("tls established");

    let mut tls_stream = StreamOwned::new(tls_conn, tcp_stream);

    tls_stream.write_all(request.as_bytes()).map_err(|e| {
        println!("could not write to stream: {e}");
        e
    })?;

    println!("wrote request to stream");

    let mut response = Vec::new();

    match tls_stream.read_to_end(&mut response) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            // harmless - the server closed without close_notify
        }
        Err(e) => return Err(e.into()),
    }

    println!("read response");

    Ok(String::from_utf8_lossy(&response).to_string())
}

fn split_response_status_line(
    _status_line: Option<&str>,
) -> Result<(&str, u16, &str), Box<dyn Error>> {
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

fn raw_http_request(request: &Request) -> String {
    let Request {
        method,
        url,
        headers,
        ..
    } = request;
    let URL { path, host, .. } = url;

    let mut request = String::new();

    let headers_to_use = with_default_headers(headers);

    request.push_str(&format!("{method} {path} HTTP/1.1\r\n"));
    request.push_str(&format!("Host: {host}\r\n"));

    for (key, value) in headers_to_use.into_iter() {
        request.push_str(&format!("{key}: {value}\r\n"));
    }

    request.push_str(&format!("\r\n"));

    request
}

fn with_default_headers(headers: &Option<Headers>) -> Headers {
    let mut default_headers: Headers = HashMap::from([
        (
            "user-agent".to_string(),
            "Mozilla/5.0 (compatible; MyBrowser/1.0; +https://github.com/agentbellnorm/browser-engineering)".to_string(),
        ),
        ("connection".to_string(), "close".to_string()),
    ]);

    default_headers.extend(headers.clone().unwrap_or(HashMap::new()));

    default_headers
}

fn get_next_request(response: &Response) -> Result<Request, Box<dyn Error>> {
    let location = response
        .headers
        .get("location")
        .expect("redirect response did not have location header");

    let original_request = response.request.clone();

    let url = match location {
        l if l.starts_with("/") => {
            let URL { scheme, host, .. } = original_request.url;
            // what about port?
            URL::parse(&format!("{scheme}://{host}{location}"))?
        }
        _ => URL::parse(location)?,
    };

    Ok(Request {
        method: Method::GET,
        url,
        headers: original_request.headers,
        body: None,
    })
}
