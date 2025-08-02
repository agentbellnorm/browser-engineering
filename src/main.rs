mod http_client;
mod url;

use crate::url::URL;
use http_client::get;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    let url_from_commandline = args.get(1);

    if let None = url_from_commandline {
        println!("no url passed to args: {:?}", args);
    }

    let url = URL::parse(url_from_commandline.unwrap()).unwrap();
    println!("{:?}", url);

    let response = get(&url, None, None).unwrap();
    println!("response: {:?}", response);

    if let Some(body) = response.body {
        let mut in_tag = false;

        for char in body.chars() {
            match (char, in_tag) {
                ('<', _) => in_tag = true,
                ('>', _) => in_tag = false,
                (c, false) => {
                    print!("{c}")
                }
                _ => {}
            }
        }
    }
}
