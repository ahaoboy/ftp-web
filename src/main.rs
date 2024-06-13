mod html;

use axum::extract::{self};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{response::Html, Router};
use clap::Parser;
use fast_qr::{QRBuilder, ECL};
use html::get_html;
use std::sync::{Arc, Mutex};
use suppaftp::FtpStream;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

#[derive(Clone)]
struct AppState {
    ftp: Arc<Mutex<FtpStream>>,
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// name
    #[arg(short, long, default_value_t = String::from(""))]
    username: String,

    /// password
    #[arg(long, short, default_value_t = String::from(""))]
    password: String,

    /// host
    #[arg(long, default_value_t = String::from("0.0.0.0"))]
    host: String,

    /// port
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// ftp
    #[clap()]
    ftp: String,
}
// static FOLDER_ICON: &str = "📁";
// static FILE_ICON: &str = "📄";

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let Cli {
        username,
        password,
        ftp,
        port,
        host,
    } = cli.clone();

    let mut ftp_stream = FtpStream::connect(&ftp).expect("can't connect to ftp server");
    ftp_stream.login(username, password).expect("login error");

    let state = AppState {
        ftp: Arc::new(Mutex::new(ftp_stream)),
    };
    let app = Router::new()
        .route("/", get(using_home))
        .route("/ftp/", get(using_home))
        .route("/ftp/*O", get(using_ftp))
        .route("/file/*O", get(using_file))
        .layer(
            CorsLayer::new()
                .allow_headers(AllowHeaders::any())
                .allow_methods(AllowMethods::any())
                .allow_origin(AllowOrigin::any()),
        )
        .fallback(using_home)
        .with_state(state);

    let port = find_port::find_port("127.0.0.1", port).expect("find port error");
    let local_ip = local_ip_address::local_ip().unwrap();

    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .unwrap();

    let s1 = format!("http://{}:{}/", "localhost", port);
    let s2 = format!("http://{}:{}/", local_ip, port);
    let s3 = format!("http://{}/{}/", host, port);
    let qrcode = QRBuilder::new(s2.clone()).ecl(ECL::H).build().unwrap();
    println!("ftp-web:\n{}\n{}\n{}", s1, s2, s3);
    qrcode.print();

    axum::serve(listener, app).await.unwrap();
}

async fn using_file(
    extract::Path(path): extract::Path<String>,
    extract::State(ftp): extract::State<AppState>,
) -> impl IntoResponse {
    let mut ftp = ftp.ftp.lock().unwrap();
    let buf = ftp.retr_as_buffer(&path).unwrap().into_inner();
    let name = path.split('/').last().unwrap_or("download");
    let headers = [(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{name}\""),
    )];
    (headers, buf).into_response()
}

async fn using_ftp(
    extract::Path(path): extract::Path<String>,
    extract::State(ftp): extract::State<AppState>,
) -> impl IntoResponse {
    let html = get_html(&ftp.ftp, &path);
    Html::from(html)
}

async fn using_home(extract::State(ftp): extract::State<AppState>) -> impl IntoResponse {
    let html = get_html(&ftp.ftp, "");
    Html::from(html)
}
