use std::cmp::min;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;

use bytes::Bytes;
use futures::{Stream, StreamExt};
use futures::future::try_join_all;
use reqwest::{Method, StatusCode};
use structopt::StructOpt;
use tokio::fs::File;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(short, long, default_value = "1")]
    threads: usize,

    url: String,

    #[structopt(short, long)]
    destination: String,
}

#[derive(Debug)]
struct ExecError(String);

impl Display for ExecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Error for ExecError {}

impl From<reqwest::Error> for ExecError {
    fn from(err: reqwest::Error) -> Self {
        ExecError(format!("network error: {}", err))
    }
}

impl From<io::Error> for ExecError {
    fn from(err: io::Error) -> Self {
        ExecError(format!("IO error: {}", err))
    }
}

#[tokio::main]
async fn main() {
    let opt = Options::from_args();

    let supports_partial_download = supports_partial_download(&opt.url).await.unwrap();

    if let (false, _) = supports_partial_download {
        return println!("url {} does not support partial download", opt.url);
    }

    if let (_, None) = supports_partial_download {
        return println!("missing content-length header");
    }

    let content_length = supports_partial_download.1.unwrap();
    let chunk_size = (content_length as f32 / opt.threads as f32).ceil() as usize;

    let mut tasks = Vec::new();

    let ranges = (0..content_length)
        .step_by(chunk_size)
        .map(|start| (start, min(start + chunk_size, content_length)));

    for (start, stop) in ranges {
        let opt = opt.clone();
        let handle = tokio::spawn(async move {
            let slug = format!("{}_{}_{}", opt.destination, start, stop);
            println!("writing chunk {}", slug);

            let mut stream = download_range(&opt.url, start, stop).await?;
            let _ = persist_range(&mut stream, &slug).await?;

            println!("chunk {} done", slug);
            Ok::<(), ExecError>(())
        });
        tasks.push(handle);
    }

    match try_join_all(tasks).await {
        Ok(_) => println!("done"),
        Err(err) => println!("error: {}", err),
    }
}

async fn persist_range(stream: &mut (impl Stream<Item=reqwest::Result<Bytes>> + Unpin), slug: &str) -> Result<(), io::Error> {
    let mut file = File::create(&slug).await?;
    while let Some(b) = stream.next().await {
        let b = b.unwrap();
        let mut b = b.as_ref();
        tokio::io::copy(&mut b, &mut file).await?;
    }
    Ok(())
}

async fn supports_partial_download(url: &str) -> reqwest::Result<(bool, Option<usize>)> {
    let client = reqwest::Client::new();
    let response = client.head(url).send().await?;
    let headers = response.headers();

    match (response.status(), headers.get("accept-ranges")) {
        (StatusCode::OK, Some(_)) => {
            let content_length = headers.get("content-length")
                .map(|hv| hv.to_str().unwrap())
                .map(|str| str.parse().unwrap());
            Ok((true, content_length))
        }
        _ => Ok((false, None))
    }
}

async fn download_range(url: &str, range_start: usize, range_stop: usize) -> reqwest::Result<impl Stream<Item=reqwest::Result<Bytes>>> {
    let client = reqwest::Client::new();
    let request = client.request(Method::GET, url)
        .header("Range", format!("bytes={}-{}", range_start, range_stop))
        .build()?;

    let stream = Box::new(client.execute(request).await?.bytes_stream());
    Ok(stream)
}
