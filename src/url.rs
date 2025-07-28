

#[derive(Debug)]
pub struct URL {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
}

impl URL {
    pub fn parse(raw: &str) -> Result<URL, &'static str> {
        let mut parts1 = raw.splitn(2, "://");
        let (scheme, rest1) = (parts1.next(), parts1.next());

        if let (Some(s), Some(mut r1)) = (scheme, rest1) {
            let r1_fixed = maybe_add_slash(r1);

            let mut parts2 = r1_fixed.splitn(2, "/");
            let (host, rest2) = (parts2.next(), parts2.next());

            if let (Some(h), Some(r2)) = (host, rest2) {
                let path = &format!("/{r2}");

                return Ok(URL {
                    scheme: s.to_string(),
                    host: h.to_string(),
                    path: path.to_string(),
                    query: None,
                    port: None,
                });
            }
        }

        Err("could not parse url")
    }
}

fn maybe_add_slash(s: &str) -> String {
    if s.contains("/") {
        return s.to_string();
    }
    return format!("{s}/");
}
