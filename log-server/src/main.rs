use bollard::Docker;
use bollard::container::ListContainersOptions;
use dotenv::dotenv;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use tar::Builder;
use warp::{Filter, Rejection, Reply};
use warp::hyper::Body;
use warp::http::Response;
use log::{info, error};

#[derive(Deserialize)]
struct LokiResponse {
    data: Data,
}

#[derive(Deserialize)]
struct Data {
    result: Vec<ResultItem>,
}

#[derive(Deserialize)]
struct ResultItem {
    stream: Stream,
    values: Vec<(String, String)>,
}

#[derive(Deserialize)]
struct Stream {
    container: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    // let log_label = env::var("LOG_LABEL").expect("LOG_LABEL must be set");
    let log_label = "config-c".to_string();
    println!("Let check the log_label: {:?}", log_label);
    info!("Starting server with LOG_LABEL: {}", log_label);

    let route = warp::path::end()
        .and(warp::get())
        .and_then(move || {
            let log_label = log_label.clone();
            async move { fetch_and_package_logs(&log_label).await }
        });

    warp::serve(route).run(([0, 0, 0, 0], 3030)).await;
}

async fn fetch_and_package_logs(log_label: &str) -> Result<impl Reply, Rejection> {
    info!("Fetching logs for label: {}", log_label);

    match fetch_and_package_logs_impl(log_label).await {
        Ok(file) => {
            info!("Successfully fetched and packaged logs.");
            let response = Response::builder()
                .header("Content-Disposition", "attachment; filename=\"logs.tar\"")
                .header("Content-Type", "application/x-tar")
                .body(file)
                .unwrap();  // Handle unwrap carefully in production code

            Ok(response)
        },
        Err(e) => {
            error!("Error fetching logs: {}", e);
            let response = Response::builder()
                .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap();  // Handle unwrap carefully in production code

            Ok(response)
        }
    }
}

async fn fetch_and_package_logs_impl(log_label: &str) -> Result<Body, Box<dyn std::error::Error>> {
    info!("Fetching logs for label: {}", log_label);

    let containers = get_containers(log_label).await?;
    info!("Found containers: {:?}", containers);

    let client = Client::new();
    let mut tar_builder = Builder::new(Vec::new());

    for container in containers {
        info!("Fetching logs for container: {}", container);
        let logs = fetch_logs(&client, &container).await?;
        tar_builder.append_data(&mut tar::Header::new_gnu(), format!("{}.log", container), logs.as_bytes())?;
    }

    let tar_data = tar_builder.into_inner()?;
    let tar_file = Body::from(tar_data);
    info!("Logs successfully packaged into tar file.");

    Ok(tar_file)
}

async fn get_containers(log_label: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    info!("Getting containers with label: {}", log_label);

    let docker = Docker::connect_with_local_defaults()?;
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![log_label.to_string()]);

    let options = Some(ListContainersOptions::<String> {
        filters,
        ..Default::default()
    });

    let containers = docker.list_containers(options).await?;
    let container_names: Vec<String> = containers.into_iter().filter_map(|container| {
        container.names.and_then(|names| names.get(0).map(|name| name.trim_start_matches('/').to_string()))
    }).collect();

    Ok(container_names)
}

async fn fetch_logs(client: &Client, container: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("http://localhost:3100/loki/api/v1/query_range?query={{container=\"{}\"}}&start=0", container);
    info!("Fetching logs from Loki: {}", url);

    let response: LokiResponse = client.get(&url).send().await?.json().await?;
    let logs: String = response.data.result.into_iter().flat_map(|item| item.values.into_iter().map(|(_, v)| v)).collect();

    info!("Fetched logs for container: {}", container);
    Ok(logs)
}
