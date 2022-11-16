use clap::Parser;
use warp::Filter;

#[derive(Parser)]
#[clap(name = "Fast-share-qr")]
#[clap(author = "Wener <aegisa7280@gmail.com>")]
#[clap(version = "0.1.0")]
#[clap(about = "Share text or file or directory to other devices by scanning a qr code", long_about = None)]
struct Args {
    ///Text you want to share
    #[clap(short, long, value_parser, group = "input", required = true)]
    text: Option<String>,
    ///File you want to share
    #[clap(short, long, value_parser, group = "input", required = true)]
    file: Option<String>,
    ///Directory you want to share
    #[clap(short, long, value_parser, group = "input", required = true)]
    directory: Option<String>,
    ///Server's port
    #[clap(short, long, value_parser)]
    port: Option<u16>,
    ///Server's hostname
    #[clap(short, long, value_parser)]
    hostname: Option<String>,
    ///Disable quiet zone of the qr code?
    #[clap(long, action)]
    disable_quiet_zone: bool,
}

#[derive(Debug)]
enum ItemType {
    Text(String),
    File(String),
    Directory(String),
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let item = (|| {
        if let Some(a) = args.text {
            return ItemType::Text(a);
        }
        if let Some(a) = args.file {
            return ItemType::File(a);
        }
        if let Some(a) = args.directory {
            return ItemType::Directory(a);
        }
        unreachable!();
    })();

    let qr_link = match item {
        ItemType::Text(t) => t,
        ItemType::File(f) => create_server(ItemType::File(f), args.hostname, args.port),
        ItemType::Directory(d) => create_server(ItemType::Directory(d), args.hostname, args.port),
    };

    let code = qrcode::QrCode::new(qr_link).unwrap();
    let string = code
        .render::<char>()
        .quiet_zone(!args.disable_quiet_zone)
        .module_dimensions(2, 1)
        .build();
    println!("{}", string);

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(16)).await;
    }
}

fn create_server(item: ItemType, hostname: Option<String>, port: Option<u16>) -> String {
    let port = port.unwrap_or_else(|| portpicker::pick_unused_port().expect("No ports free"));
    let hostname = hostname.unwrap_or_else(|| local_ipaddress::get().expect("Can't get local ip"));
    tokio::spawn(async move {
        match item {
            ItemType::File(a) => {
                let file_name = String::from(a.split('/').rev().next().unwrap());
                let headers = warp_headers_for_downloading_file(&file_name);
                let routes = warp::get()
                    .and(warp::path::end())
                    .and(warp::fs::file(file_name))
                    .with(warp::reply::with::headers(headers));
                warp::serve(routes).run(([0, 0, 0, 0], port)).await;
            }
            ItemType::Directory(a) => {
                let routes =
                    warp::any()
                        .and(warp::path::tail())
                        .map(move |p: warp::filters::path::Tail| {
                            if p.as_str() == "favicon.ico" {
                                return warp::http::Response::builder()
                                    .status(404)
                                    .body(Vec::new())
                                    .unwrap();
                            }
                            let p = format!("{}/{}", &a, urlencoding::decode(p.as_str()).unwrap());
                            if !std::path::Path::new(&p).exists() {
                                return warp::http::Response::builder()
                                    .status(404)
                                    .body(Vec::new())
                                    .unwrap();
                            }
                            let p = std::path::Path::new(&p);
                            if p.is_dir() {
                                use std::io::Write;
                                let mut body = Vec::new();
                                for item in std::fs::read_dir(p).unwrap() {
                                    let item = item.unwrap();
                                    write!(
                                        &mut body,
                                        "<a href=\"/{}\">{}</a><br />",
                                        item.path().strip_prefix(&a).unwrap().to_str().unwrap(),
                                        item.file_name().to_str().unwrap()
                                    )
                                    .unwrap();
                                }
                                return warp::http::Response::builder().body(body).unwrap();
                            } else if p.is_file() {
                                use std::io::prelude::*;
                                let mut file = std::fs::File::open(p).unwrap();
                                let mut buffer = Vec::new();
                                file.read_to_end(&mut buffer).unwrap();
                                return warp::http::Response::builder()
                                    .header("Content-Type", "application/octet-stream")
                                    .header(
                                        "Content-Disposition",
                                        a.split('/').rev().next().unwrap(),
                                    )
                                    .body(buffer)
                                    .unwrap();
                            } else if p.is_symlink() {
                                todo!()
                            }
                            unreachable!()
                        });
                warp::serve(routes).run(([0, 0, 0, 0], port)).await;
            }
            ItemType::Text(_) => unreachable!(),
        }
    });
    format!("http://{hostname}:{port}/")
}

//Example input: hello_world.rs
//Example output: headers containing: Content-Type: application/octet-stream
//                                    Content-Disposition: attachment;filename=hello_world.rs
fn warp_headers_for_downloading_file(file_name: &String) -> warp::http::header::HeaderMap {
    let mut headers = warp::http::header::HeaderMap::new();
    headers.insert(
        "Content-Type",
        warp::http::header::HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        "Content-Disposition",
        warp::http::header::HeaderValue::from_str(&format!("attachment;filename={}", file_name))
            .unwrap(),
    );
    headers
}
