use axum::extract::{self, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect};
use fileinfo::FileInfo;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::{str, vec};
use suppaftp::FtpStream;

use axum::{response::Html, routing::get, Router};
// type AppState = Mutex<Arc<FtpStream>>;
#[derive(Clone)]
struct AppState {
    ftp: Arc<Mutex<FtpStream>>,
}

#[tokio::main]
async fn main() {
    let mut ftp_stream = FtpStream::connect("192.168.0.64:7266").unwrap();
    let _ = ftp_stream.login("pc", "111111").unwrap();
    let state = AppState {
        ftp: Arc::new(Mutex::new(ftp_stream)),
    };
    let app = Router::new()
        .route("/", get(|| async { Redirect::permanent("/ftp/") }))
        .route("/ftp/*O", get(using_ftp))
        .route("/file/*O", get(using_file))
        // .route("/*", get(|| async { Redirect::permanent("/ftp/") }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://{}/", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn using_file(
    extract::Path(path): extract::Path<String>,
    extract::State(ftp): extract::State<AppState>,
) -> impl IntoResponse {
    let mut ftp = ftp.ftp.lock().unwrap();
    let buf = ftp.retr_as_buffer(&path).unwrap().into_inner();
    let name = path.split("/").last().unwrap_or("download");
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
    // println!("using_ftp: {:?}", path);
    let html = get_html(&ftp.ftp, &path);
    return Html::from(html);
}

fn get_html(ftp: &Arc<Mutex<FtpStream>>, path: &str) -> String {
    let mut ftp = ftp.lock().unwrap();
    let list = ftp.list(Some(&path)).or(ftp.list(None)).unwrap();

    let li_text = list.iter().filter_map(|i| {
        let info: FileInfo = FileInfo::from_str(&i).ok()?;
        let name = &info.name;
        if info.is_dir() {
            return Some(format!("<li><a href='/ftp/{path}/{name}/'>{name}</a></li>",));
        }
        return Some(format!(
            "<li><a target='_blank' href='/file/{path}/{name}'>{name}</a></li>",
        ));
    });

    let li_text = li_text.collect::<Vec<_>>();
    let li_text = li_text.join("\n");

    let html = format!(
        r#"
<html><head>
    <meta charset='utf-8'>
      <title>Index of {path}</title></head><body><h1>Index of ${path}</h1>
    <ul>
{li_text}
    </ul>
    </body>
</html>"#
    );

    html
}
