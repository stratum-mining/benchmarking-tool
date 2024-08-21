use bollard::container::ListContainersOptions;
use bollard::Docker;
use dotenv::dotenv;
use log::{error, info};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use tar::Builder;
use warp::http::Response;
use warp::hyper::Body;
use warp::{Filter, Rejection, Reply};

#[derive(Deserialize, Debug)]
struct LokiResponse {
    data: Data,
}

#[derive(Deserialize, Debug)]
struct Data {
    result: Vec<ResultItem>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ResultItem {
    stream: Stream,
    values: Vec<(String, String)>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct Stream {
    container: String,
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let log_label = env::var("LOG_LABEL").expect("LOG_LABEL must be set");
    let log_label = format!("logging={}", log_label);
    info!("Starting server with LOG_LABEL: {}", log_label);

    let route = warp::path::end().and(warp::get()).and_then(move || {
        let log_label = log_label.clone();
        async move { fetch_and_package_logs(&log_label).await }
    });

    warp::serve(route).run(([0, 0, 0, 0], 7420)).await;
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
                .unwrap();

            Ok(response)
        }
        Err(e) => {
            error!("Error fetching logs: {}", e);
            let response = Response::builder()
                .status(warp::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap();

            Ok(response)
        }
    }
}

async fn fetch_and_package_logs_impl(log_label: &str) -> Result<Body, Box<dyn std::error::Error>> {
    info!("Fetching logs for label: {}", log_label);

    let containers = get_containers(log_label).await?;
    info!("Found containers: {:?}", containers);

    let client = Client::new();
    let mut tar_data = Vec::new();
    {
        let mut tar_builder = Builder::new(&mut tar_data);

        let readme_content = r#"
### Download Logs from the Grafana Dashboard

We’ve added a feature that allows you to download logs of all containers directly from the Grafana dashboard. Here’s how to use it:

1. Navigate to the Grafana dashboard.
2. Look for the 'Download Logs' button and click on it.
3. The logs will be downloaded as a .tar file.

To check logs of the containers and if facing any issues and want help, kindly share the logs in the [benchmarking channel on Discord](https://discord.com/channels/950687892169195530/1107964065936060467).
"#;
        let mut header = tar::Header::new_gnu();
        header.set_size(readme_content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar_builder.append_data(&mut header, "README.md", readme_content.as_bytes())?;

        for container in containers {
            info!("Fetching logs for container: {}", container);
            match fetch_logs(&client, &container).await {
                Ok(logs) => {
                    let mut header = tar::Header::new_gnu();
                    header.set_size(logs.len() as u64);
                    header.set_mode(0o644);
                    header.set_cksum();
                    if let Err(e) = tar_builder.append_data(
                        &mut header,
                        format!("{}.log", container),
                        logs.as_bytes(),
                    ) {
                        error!("Failed to append data for container {}: {}", container, e);
                    }
                }
                Err(e) => {
                    error!("Failed to fetch logs for container {}: {}", container, e);
                }
            }
        }

        tar_builder.finish()?;
    }
    info!("Logs successfully packaged into tar file.");

    let tar_file = Body::from(tar_data);
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
    let container_names: Vec<String> = containers
        .into_iter()
        .filter_map(|container| {
            container.names.and_then(|names| {
                names
                    .first()
                    .map(|name| name.trim_start_matches('/').to_string())
            })
        })
        .collect();

    Ok(container_names)
}

async fn fetch_logs(
    client: &Client,
    container: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "http://loki:3100/loki/api/v1/query_range?query={{container=\"{}\"}}&limit=100000000",
        container
    );
    info!("Fetching logs from Loki: {}", url);

    let response: LokiResponse = client.get(&url).send().await?.json().await?;
    let mut logs: Vec<(String, String)> = response
        .data
        .result
        .into_iter()
        .flat_map(|item| item.values.into_iter())
        .collect();

    logs.sort_by(|a, b| a.0.cmp(&b.0));

    let formatted_logs: String = logs
        .into_iter()
        .fold(String::new(), |mut acc, (_, message)| {
            writeln!(&mut acc, "{}", message).unwrap();
            acc
        });

    info!("Fetched logs for container: {}", container);
    Ok(formatted_logs)
}
