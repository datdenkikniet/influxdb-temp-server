mod client;

use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, get_service},
    Extension, Router,
};

use clap::Parser;
use client::Client;
use duration_string::DurationString;
use serde::Serialize;
use tokio::sync::Mutex;
use tower_http::{add_extension::AddExtensionLayer, services::ServeDir};

#[derive(Parser)]
struct Opts {
    #[clap(env = "INFLUXDB_TOKEN")]
    pub api_token: String,
    #[clap(env = "INFLUXDB_HOST")]
    pub host: String,
    #[clap(env = "INFLUXDB_ORG")]
    pub org: String,

    #[clap(env = "HTTP_PORT", default_value = "3000")]
    pub http_port: u32,
}

type SharedState = Arc<Mutex<Client>>;

#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    run(opts).await;
}

async fn run(opts: Opts) {
    let client = influxdb2::Client::new(opts.host, opts.org, opts.api_token);
    let mut client = Client::new(client);

    client.get_current_temp().await.unwrap();
    client
        .get_temps_in_span(Duration::from_secs(1000))
        .await
        .unwrap()
        .next();

    let client = Arc::new(Mutex::new(client));

    let app = Router::new()
        .route("/temp/current", get(current_temp))
        .route("/temp/range/:range", get(temp_range))
        .route("/temp/from/:start/to/:stop", get(temp_range_start_end))
        .route("/humidity/range/:range", get(humidity_range))
        .route(
            "/humidity/from/:start/to/:stop",
            get(humidity_range_start_end),
        )
        .fallback(get_service(ServeDir::new("./static")).handle_error(handle_error))
        .layer(AddExtensionLayer::new(client));

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

async fn current_temp(Extension(client): Extension<SharedState>) -> impl IntoResponse {
    match client.lock().await.get_current_temp().await {
        Some(temp) => (StatusCode::OK, format!("{:.02}", temp.value)),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not get current temperature".to_string(),
        ),
    }
}

fn to_json<S: Serialize>(input: Vec<S>) -> (StatusCode, String) {
    let start = Instant::now();
    let output = match serde_json::to_string(&input) {
        Ok(v) => v,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
    };
    println!("Took {} ms to serialize", start.elapsed().as_millis());

    (StatusCode::OK, output)
}

async fn temp_range_start_end(
    Path((start, stop)): Path<(u64, u64)>,
    Extension(client): Extension<SharedState>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let temps: Vec<_> = match client.lock().await.get_temps_from_to(start, stop).await {
        Ok(v) => v.collect(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
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

async fn temp_range(
    Path(path): Path<String>,
    Extension(client): Extension<SharedState>,
) -> impl IntoResponse {
    let duration = match get_range(&path) {
        Ok(duration) => duration.into(),
        Err(e) => return e,
    };

    let start = Instant::now();
    let temps: Vec<_> = match client.lock().await.get_temps_in_span(duration).await {
        Ok(v) => v.collect(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
    };

    println!(
        "Took {} ms to fetch {} temperature measurements",
        start.elapsed().as_millis(),
        temps.len()
    );

    to_json(temps)
}

async fn humidity_range(
    Path(path): Path<String>,
    Extension(client): Extension<SharedState>,
) -> impl IntoResponse {
    let duration = match get_range(&path) {
        Ok(duration) => duration.into(),
        Err(e) => return e,
    };

    let start = Instant::now();
    let humidities: Vec<_> = match client.lock().await.get_hums_in_span(duration).await {
        Ok(v) => v.collect(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
    };

    println!(
        "Took {} ms to fetch {} humidity measurements",
        start.elapsed().as_millis(),
        humidities.len()
    );

    to_json(humidities)
}

async fn humidity_range_start_end(
    Path((start, stop)): Path<(u64, u64)>,
    Extension(client): Extension<SharedState>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let temps: Vec<_> = match client.lock().await.get_hums_from_to(start, stop).await {
        Ok(v) => v.collect(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")),
    };

    println!(
        "Took {} ms to fetch {} humidity measurements",
        start_time.elapsed().as_millis(),
        temps.len()
    );

    to_json(temps)
}
