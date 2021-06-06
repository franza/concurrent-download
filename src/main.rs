use std::error::Error;

use bytes::Bytes;
use reqwest::{Method, StatusCode};
use tokio::fs::File;
use futures::future::join_all;
use structopt::{StructOpt};
use futures::StreamExt;

#[derive(Debug, StructOpt, Clone)]
struct Options {
    #[structopt(short, long, default_value = "1")]
    threads: usize,

    url: String,

    #[structopt(short, long)]
    destination: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Options::from_args();

    let supports_partial_download = supports_partial_download(&opt.url).await?;

    if let (false, _) = supports_partial_download {
        return Ok(println!("url {} does not support partial download", opt.url));
    }

    if let (_, None) = supports_partial_download {
        return Ok(println!("missing content-length header"));
    }

    let content_length = supports_partial_download.1.unwrap();

    let chunk_size = (content_length as f32 / opt.threads as f32).ceil() as usize;

    let mut tasks = Vec::new();

    for start in (0..content_length).step_by(chunk_size) {
        let opt = opt.clone();
        let handle = tokio::spawn(async move {
            let stop = std::cmp::min(start + chunk_size, content_length);
            println!("downloading chunk {}-{} of {}", start, stop, content_length);
            match download_and_copy(&opt.url, start, stop, &opt.destination).await {
                Ok(_) => println!("chunk {}-{} done", start, stop),
                Err(err) => println!("error: {}", err),
            }
        });
        tasks.push(handle);
    }
    join_all(tasks).await;

    Ok(println!("done"))
}

async fn download_and_copy(url: &str, start: usize, stop: usize, destination: &str) -> Result<String, Box<dyn Error>> {
    let mut bytes = download_range(&url, start, stop).await?;
    let filename = format!("{}_{}_{}", destination, start, stop);
    let mut file = File::create(&filename).await?;
    while let Some(b) = (bytes).next().await {
        let b = b.unwrap();
        let mut b = b.as_ref();
        tokio::io::copy(&mut b, &mut file).await?;
    }
    Ok(filename)
}

async fn supports_partial_download(url: &str) -> Result<(bool, Option<usize>), Box<dyn Error>> {
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

async fn download_range(url: &str, range_start: usize, range_stop: usize) -> Result<impl futures::Stream<Item=reqwest::Result<Bytes>>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let request = client.request(Method::GET, url)
        .header("Range", format!("bytes={}-{}", range_start, range_stop))
        .build()?;

    let stream = Box::new(client.execute(request).await?.bytes_stream());
    Ok(stream)
}
