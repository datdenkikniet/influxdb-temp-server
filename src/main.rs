mod client;

use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::Path,
    headers::{authorization::Bearer, Authorization},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Extension, Router, TypedHeader,
};

use clap::Parser;
use client::Client;
use duration_string::DurationString;
use serde::Serialize;
use tower_http::{
    add_extension::AddExtensionLayer, compression::CompressionLayer, services::ServeDir,
};

#[derive(Parser)]
struct Opts {
    #[clap(env = "INFLUXDB_TOKEN")]
    pub api_token: String,
    #[clap(env = "INFLUXDB_HOST")]
    pub host: String,
    #[clap(env = "INFLUXDB_ORG")]
    pub org: String,
    #[clap(env = "HTTP_PASSWORD")]
    pub http_password: String,
    #[clap(env = "HTTP_PORT", default_value = "3000")]
    pub http_port: u32,
}

type SharedState = Arc<Client>;

#[derive(Debug, Clone)]
struct HttpPassword(String);

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    run(opts).await;
}

async fn run(opts: Opts) {
    let client = influxdb2::Client::new(opts.host, opts.org, opts.api_token);
    let client = Client::new(client);

    client.get_current().await.unwrap();
    client
        .get_data_in_span(Duration::from_secs(1000))
        .await
        .unwrap()
        .next();

    let client = Arc::new(client);

    let brotli = CompressionLayer::new().no_gzip().no_deflate();
    let other_compression = CompressionLayer::new().no_br();

    let app = Router::new()
        .route("/temp/current", get(current_temp))
        .route("/data/range/:range", get(data_range))
        .route("/data/from/:start/to/:stop", get(data_range_start_end))
        .fallback(get_service(ServeDir::new("./static")).handle_error(handle_error))
        .layer(AddExtensionLayer::new(client))
        .layer(AddExtensionLayer::new(HttpPassword(opts.http_password)))
        // Put brotli in it's own layer because it compresses this JSON data
        // a lot better but for some reason the CompressionLayer prefers gzip
        // and deflate over it.
        .layer(brotli)
        .layer(other_compression);

    let addr = format!("[::]:{}", opts.http_port).parse().unwrap();

    println!("Starting server on port {}", opts.http_port);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_error(_err: std::io::Error) -> impl axum::response::IntoResponse {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        "Something went wrong...",
    )
}

async fn check_password(
    password: String,
    input: TypedHeader<Authorization<Bearer>>,
) -> Result<(), (axum::http::StatusCode, String)> {
    let input_password = input.token();

    if password != input_password {
        return Err((StatusCode::UNAUTHORIZED, "Invalid password".to_string()));
    } else {
        Ok(())
    }
}

async fn current_temp(Extension(client): Extension<SharedState>) -> impl IntoResponse {
    match client.get_current().await {
        Some(data) => Ok(format!("{:.02}", data.temperature)),
        None => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not get current temperature".to_string(),
        )),
    }
}

fn to_json<S: Serialize>(input: Vec<S>) -> Result<String, (StatusCode, String)> {
    let start = Instant::now();
    let output = match serde_json::to_string(&input) {
        Ok(v) => v,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))),
    };
    println!("Took {} ms to serialize", start.elapsed().as_millis());

    Ok(output)
}

async fn data_range(
    Path(path): Path<String>,
    Extension(client): Extension<SharedState>,
    Extension(HttpPassword(password)): Extension<HttpPassword>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    check_password(password, auth).await?;
    let duration = match get_range(&path) {
        Ok(duration) => duration.into(),
        Err(e) => return Err(e),
    };

    let start = Instant::now();
    let temps: Vec<_> = match client.get_data_in_span(duration).await {
        Ok(v) => v.collect(),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))),
    };

    println!(
        "Took {} ms to fetch {} temperature measurements",
        start.elapsed().as_millis(),
        temps.len()
    );

    to_json(temps)
}

async fn data_range_start_end(
    Path((start, stop)): Path<(u64, u64)>,
    Extension(client): Extension<SharedState>,
    Extension(HttpPassword(password)): Extension<HttpPassword>,
    auth: TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    check_password(password, auth).await?;

    let start_time = Instant::now();
    let temps: Vec<_> = match client.get_data_from_to(start, stop).await {
        Ok(v) => v.collect(),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))),
    };

    println!(
        "Took {} ms to fetch {} temperature measurements",
        start_time.elapsed().as_millis(),
        temps.len()
    );

    to_json(temps)
}

fn get_range(input: &str) -> Result<Duration, (StatusCode, String)> {
    match DurationString::from_str(&input) {
        Ok(duration) => Ok(duration.into()),
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            format!("Could not convert {input} into a duration ({e})."),
        )),
    }
}
