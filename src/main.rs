use warp::serve;
use warp::Filter;
use warp::reply::html;
use warp::Reply;
use tokio::fs::File;
use tokio::fs::read_dir;
use tokio_util::io::StreamReader;
use futures_util::stream::StreamExt;
use http::status::StatusCode;

// For multipart
use bytes::Buf;
use futures::stream::TryStreamExt;
use futures::Stream;
use mime::Mime;
use mpart_async::server::MultipartStream;

use serde::Serialize;

use std::convert::Infallible;
use std::io::Error as IoError;
use std::str::FromStr;
use std::fmt::Display;
use std::process::exit;
use std::env::args;
use std::env::current_dir;

const HTML_INDEX: &str = include_str!("./index.html");
const HTML_NOT_FOUND: &str = include_str!("./not-found.html");
const DEFAULT_MAX_UPLOAD_SIZE: u64 = 1024 * 1024 * 500;
const DEFAULT_PORT: u16 = 8000;

#[tokio::main]
async fn main() {
    let mut arguments = args();
    let program = arguments.next().unwrap();
    let port: u16 = get_arg(arguments.next(), DEFAULT_PORT, &program);
    let max_upload_size: u64 = get_arg(arguments.next(), DEFAULT_MAX_UPLOAD_SIZE, &program);
    let cwd = current_dir().unwrap().to_str().unwrap().to_owned();

    let upload_handler = warp::path!("api" / "upload")
        .and(warp::header::<Mime>("content-type"));
    let upload_handler = if max_upload_size != 0 {
        upload_handler
          .and(warp::body::content_length_limit(max_upload_size))
          .boxed()
    } else {
      upload_handler.boxed()
    };

    let routes = warp::path::end()
        .or(warp::path("list"))
            .map(|_| html(HTML_INDEX))
        .or(upload_handler
            .and(warp::body::stream())
            .and_then(write_file))
        .or(warp::path!("api" / "list")
            .and_then(list_files))
        .or(warp::path!("api" / "max-size")
            .map(move || max_upload_size.to_string()))
        .or(warp::path("api")
            .and(warp::path("dl"))
            .and(warp::fs::dir(cwd)))
        .recover(|_| async move {
            Result::<_, Infallible>::Ok(warp::reply::with_status(warp::reply::html(HTML_NOT_FOUND), StatusCode::NOT_FOUND))
        });

    println!("Listening on 127.0.0.1:{}", port);
    serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}

fn get_arg<T: FromStr + Display>(arg_raw: Option<String>, default: T, program: &str) -> T {
    if let Some(arg) = arg_raw {
        if let Ok(size) = arg.parse() { size } else {
            print!(
"USAGE
  {} [PORT [MAX_UPLOAD]]

DESCRIPTION
  Simple HTTP-based file upload service.

OPTIONS
  PORT
    Port to listen on. Defaults to {}

  MAX_UPLOAD
    Maximum file upload size in bytes. Defaults to {}MB. Set
    to zero to disable size requirements.

Help: You might have provided MAX_UPLOAD before PORT
",
                program,
                DEFAULT_PORT,
                DEFAULT_MAX_UPLOAD_SIZE / 1024 / 1024,
            );
            exit(1);
        }
    } else { default }
}

async fn write_file(
    mime: Mime,
    body: impl Stream<Item = Result<impl Buf, warp::Error>> + Unpin,
) -> Result<impl warp::Reply, Infallible> {
    let boundary = mime.get_param("boundary").map(|v| v.to_string()).unwrap();

    let mut stream = MultipartStream::new(
        boundary,
        body.map_ok(|mut buf| buf.copy_to_bytes(buf.remaining())),
    );

    while let Ok(Some(field)) = stream.try_next().await {
        let filename = field.filename().unwrap().to_owned();
        let mut file = File::create(filename.as_str()).await.unwrap();
        let mut read_stream = StreamReader::new(field.map(|x| Result::<_, IoError>::Ok(x.unwrap())));
        tokio::io::copy_buf(&mut read_stream, &mut file).await.unwrap();
        println!("Uploaded '{}'", filename);
    }

    Ok("Done")
}

#[derive(Serialize)]
struct ListEntry {
    name: String,
    is_file: bool,
    size: u64,
}

async fn list_files() -> Result<impl Reply, Infallible> {
    let mut entries: Vec<ListEntry> = vec![];
    let mut dir_entries = read_dir(".").await.unwrap();

    while let Ok(Some(entry)) = dir_entries.next_entry().await {
        let metadata = entry.metadata().await.unwrap();
        entries.push(ListEntry {
            name: entry.file_name().to_str().unwrap().into(),
            size: metadata.len(),
            is_file: metadata.is_file(),
        });
    }

    Ok(serde_json::to_string(&entries).unwrap())
}

