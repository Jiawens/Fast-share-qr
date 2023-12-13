use clap::Parser;
use std::path::Path;
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
        //Moving port & hostname into it
        match item {
            ItemType::File(a) => {
                let file_name = Path::new(&a).file_name().unwrap().to_str().unwrap();
                let headers = warp_headers_for_downloading_file(&file_name.to_owned());
                //This route match /the/file_you_want_to.download
                //And then send this file
                let routes = warp::get()
                    .and(warp::path::end())
                        .and(warp::fs::file(std::path::PathBuf::from(a)))
                    .with(warp::reply::with::headers(headers));
                warp::serve(routes).run(([0, 0, 0, 0], port)).await;
            }
            ItemType::Directory(a) => {
                //This route match:
                //1. /the/folder%20you_want_to_look_through
                //2. /the/file%20you_want_to.download
                let routes = warp::any()
                    .and(warp::path::tail()) //So we can process this url ourselves
                    .map(move |url: warp::filters::path::Tail| {
                        //Check if browser is requesting favicon.ico
                        if url.as_str() == "favicon.ico" {
                            return warp_404_not_found_response();
                        }
                        //Browser will turn spaces to %20, so we need to decode them
                        let url: String = urlencoding::decode(url.as_str()).unwrap().into();
                        //Add given prefix(a) to left of url
                        //Example:
                        //a contains "/usr", url contains "bin", then url will be "/usr/bin"
                        let path = Path::new(&a).join(Path::new(&url));
                        //Let's return a 404 when the path dosen't exist
                        if !path.exists() {
                            return warp_404_not_found_response();
                        }
                        if path.is_dir() {
                            use std::io::Write;
                            let mut body = Vec::new();
                            for item in std::fs::read_dir(path).unwrap() {
                                let item = item.unwrap();
                                //Example output: <a href="/usr/share">share</a><br />
                                write!(
                                    &mut body,
                                    "<a href=\"/{}\">{}</a><br />",
                                    //Strip the given prefix(a) to avoid additional prefix
                                    //Example:
                                    //a contains "/usr", item.path() contains "/usr/share"
                                    //then output will be "/share"
                                    item.path().strip_prefix(&a).unwrap().to_str().unwrap(),
                                    item.file_name().to_str().unwrap()
                                )
                                .unwrap();
                            }
                            return warp::http::Response::builder().body(body).unwrap();
                        } else if path.is_file() {
                            use std::io::prelude::*;
                            let mut file = std::fs::File::open(path).unwrap();
                            let mut buffer = Vec::new();
                            file.read_to_end(&mut buffer).unwrap();
                            return warp::http::Response::builder()
                                .header("Content-Type", "application/octet-stream")
                                .header("Content-Disposition", a.split('/').rev().next().unwrap())
                                .body(buffer)
                                .unwrap();
                        } else if path.is_symlink() {
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
    use warp::http::header::{HeaderMap, HeaderValue};

    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        "Content-Disposition",
        HeaderValue::from_str(&format!("attachment;filename={}", file_name)).unwrap(),
    );
    headers
}

//Return a 404 response with empty body
fn warp_404_not_found_response() -> warp::http::Response<Vec<u8>> {
    warp::http::Response::builder()
        .status(404)
        .body(Vec::new())
        .unwrap()
}
