use warp::serve;
use warp::Filter;
use warp::reply::html;
use warp::Reply;
use warp::Error;
use warp::multipart::{FormData, Part, form};
use tokio::fs::File;
use tokio::fs::read_dir;
use tokio_util::io::StreamReader;
use futures_util::stream::TryStreamExt;
use futures_util::stream::StreamExt;
use http::status::StatusCode;

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

    let routes = warp::path::end()
        .or(warp::path("list"))
            .map(|_| html(HTML_INDEX))
        .or(warp::path!("api" / "upload")
            .and(warp::post())
            .and(form().max_length(max_upload_size))
            .and(warp::body::content_length_limit(max_upload_size))
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
    Maximum file upload size in bytes. Defaults to {}MB
",
                program,
                DEFAULT_PORT,
                DEFAULT_MAX_UPLOAD_SIZE / 1024 / 1024,
            );
            exit(1);
        }
    } else { default }
}

async fn write_file(form_data: FormData) -> Result<impl Reply, Infallible> {
    let names: Vec<String> = form_data
        .try_filter_map(process_form_part)
        .try_collect()
        .await.unwrap();
    println!("Uploaded '{}'", names.get(0).unwrap_or(&"no name".to_owned()));
    Ok(format!("Received {} things successfully", names.len()))
}

async fn process_form_part(part: Part) -> Result<Option<String>, Error> {
    let name = part.filename().unwrap().to_owned();
    let mut file = File::create(name.as_str()).await.unwrap();
    let mut read_stream = StreamReader::new(part.stream().map(|x| Result::<_, IoError>::Ok(x.unwrap())));
    tokio::io::copy_buf(&mut read_stream, &mut file).await.unwrap();
    Ok(Some(name))
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

