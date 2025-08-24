use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct URL {
    pub scheme: String,
    pub host: String,
    pub port: Option<String>,
    pub path: String,
    pub query: Option<String>,
}

#[test]
fn parse_localhost() {
    assert_eq!(
        URL::parse("http://localhost:3000/lol.html").expect("hej"),
        URL {
            scheme: "http".to_string(),
            host: "localhost".to_string(),
            port: Some("3000".to_string()),
            path: "/lol.html".to_string(),
            query: None,
        }
    )
}

#[test]
fn parse_http() {
    assert_eq!(
        URL::parse("http://example.com/hej").expect("hej"),
        URL {
            scheme: "http".to_string(),
            host: "example.com".to_string(),
            port: None,
            path: "/hej".to_string(),
            query: None,
        }
    )
}

#[test]
fn parse_https() {
    assert_eq!(
        URL::parse("https://example.com/hej").expect("hej"),
        URL {
            scheme: "https".to_string(),
            host: "example.com".to_string(),
            port: None,
            path: "/hej".to_string(),
            query: None,
        }
    )
}

impl URL {
    pub fn parse(raw: &str) -> Result<URL, Box<dyn Error>> {
        let mut parts1 = raw.splitn(2, "://");
        let (scheme, rest1) = (parts1.next(), parts1.next());

        if let (Some(s), Some(mut r1)) = (scheme, rest1) {
            let r1_fixed = maybe_add_slash(r1);

            let mut parts2 = r1_fixed.splitn(2, "/");
            let (host, rest2) = (parts2.next(), parts2.next());

            if let (Some(mut h), Some(r2)) = (host, rest2) {
                let path = &format!("/{r2}");
                let mut port: Option<String> = None;

                if h.contains(":") {
                    let mut parts3 = h.splitn(2, ":");
                    let (host_without_port, parsed_port) = (parts3.next(), parts3.next());
                    port = parsed_port.map(|slice| slice.to_string());
                    h = host_without_port.ok_or("no host")?;
                }

                return Ok(URL {
                    scheme: s.to_string(),
                    host: h.to_string(),
                    path: path.to_string(),
                    query: None,
                    port,
                });
            }
        }

        Err(format!("could not parse url: {raw}").into())
    }

    pub fn domain(&self) -> String {
        let port_to_use = match self.scheme.as_str() {
            "http" => self.port.clone().unwrap_or("80".to_string()),
            "https" => self.port.clone().unwrap_or("443".to_string()),
            _ => todo!("unsupported scheme"),
        };

        let host = self.host.clone();

        format!("{host}:{port_to_use}")
    }
}

fn maybe_add_slash(s: &str) -> String {
    if s.contains("/") {
        return s.to_string();
    }
    return format!("{s}/");
}
