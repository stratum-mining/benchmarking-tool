use demand_easy_sv2::const_sv2::{
    MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS, MESSAGE_TYPE_SUBMIT_SHARES_ERROR, MESSAGE_TYPE_SUBMIT_SOLUTION,
};
use demand_easy_sv2::roles_logic_sv2::parsers::{Mining, PoolMessages, TemplateDistribution};
use demand_easy_sv2::{ProxyBuilder, Remote};
use prometheus::{register_counter, register_gauge, register_gauge_vec, Counter, Encoder, Gauge, GaugeVec, TextEncoder};
use std::env;
use std::net::ToSocketAddrs;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use warp::Filter;
use reqwest::Client;

#[tokio::main]
async fn main() {
    // Load environment variables
    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");
    let proxy_type = env::var("PROXY_TYPE").expect("PROXY_TYPE environment variable not set");
    let prometheus_exporter_address = env::var("PROM_ADDRESS").expect("PROM_ADDRESS environment variable not set");

    // Declare variables for metrics
    let mut submitted_shares: Option<Counter> = None;
    let mut valid_shares: Option<Counter> = None;
    let mut stale_shares: Option<Counter> = None;
    let mut share_submission_timestamp: Option<GaugeVec> = None;
    let mut block_propagation_through_jdc_latency: Option<Gauge> = None;
    let mut block_propagation_through_pool_latency: Option<Gauge> = None;

    // Initialize metrics based on proxy_type
    match proxy_type.as_str() {
        "tp-jdc" => {
            block_propagation_through_jdc_latency = Some(register_gauge!(
                "block_propagation_through_jdc_latency",
                "Latency for submit solution through JDC in milliseconds",
            ).unwrap());
        },
        "tp-pool" => {
            block_propagation_through_pool_latency = Some(register_gauge!(
                "block_propagation_through_pool_latency",
                "Latency for submit solution through Pool in milliseconds",
            ).unwrap());
        },
        "pool-translator" | "jdc-translator" => {
            submitted_shares = Some(register_counter!(
                "sv2_submitted_shares",
                "Total number of SV2 submitted shares"
            ).unwrap());
            valid_shares = Some(register_counter!(
                "sv2_valid_shares",
                "Total number of SV2 valid shares"
            ).unwrap());
            stale_shares = Some(register_counter!(
                "sv2_stale_shares",
                "Total number of SV2 stale shares"
            ).unwrap());
            share_submission_timestamp = Some(register_gauge_vec!(
                "share_submission_timestamp",
                "Timestamp of the submitted share",
                &["nonce"]
            ).unwrap());
        },
        _ => panic!("Invalid PROXY_TYPE"),
    }

    // Spawn the metrics endpoint
    tokio::spawn(async move {
        let metrics_route = warp::path("metrics").map(move || {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = Vec::new();
            encoder.encode(&metric_families, &mut buffer).unwrap();
            warp::http::Response::builder()
                .header("Content-Type", encoder.format_type())
                .body(buffer)
        });
        let addr: std::net::SocketAddr = prometheus_exporter_address.parse().expect("Invalid address");
        warp::serve(metrics_route).run(addr).await;
    });

    let mut proxy_builder = ProxyBuilder::new();
    proxy_builder
        .try_add_client(listen_for_client(&client_address).await)
        .await
        .unwrap()
        .try_add_server(connect_to_server(&server_address).await)
        .await
        .unwrap();

    // Handle proxy type specific logic
    match proxy_type.as_str() {
        "pool-translator" | "jdc-translator" => {
            if let (Some(shares), Some(valid), Some(stale), Some(timestamp)) = (submitted_shares, valid_shares, stale_shares, share_submission_timestamp) {
                intercept_submit_share_extended(&mut proxy_builder, shares.clone(), timestamp.clone()).await;
                intercept_submit_share_success(&mut proxy_builder, valid.clone()).await;
                intercept_submit_share_error(&mut proxy_builder, stale.clone()).await;
            }
        },
        "tp-pool" => {
            if let Some(pool_latency) = block_propagation_through_pool_latency {
                intercept_submit_solution(&mut proxy_builder, pool_latency).await;
            }
        },
        "tp-jdc" => {
            if let Some(jdc_latency) = block_propagation_through_jdc_latency {
                intercept_submit_solution(&mut proxy_builder, jdc_latency).await;
            }
        },
        _ => {
            panic!("Invalid PROXY_TYPE");
        },
    }

    let proxy = proxy_builder.try_build().unwrap();
    let _ = proxy.start().await;
}

async fn listen_for_client(client_address: &str) -> TcpStream {
    let address = client_address.to_socket_addrs().unwrap().next().unwrap();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    if let Ok((stream, _)) = listener.accept().await {
        stream
    } else {
        panic!()
    }
}

async fn connect_to_server(server_address: &str) -> TcpStream {
    let address = server_address.to_socket_addrs().unwrap().next().unwrap();
    TcpStream::connect(address).await.unwrap()
}

async fn intercept_submit_share_extended(
    builder: &mut ProxyBuilder,
    submitted_shares: Counter,
    gauge: GaugeVec,
) {
    let mut r = builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesExtended(m))) = r.recv().await {
            println!("SubmitSharesExtended received --> {:?}", m);
            submitted_shares.inc();
            
            let id = m.nonce;
            let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;

            // Clone the gauge for use in the task
            let gauge_clone = gauge.clone();
            gauge_clone.with_label_values(&[&id.to_string()]).set(current_time);

            // Spawn a task to remove the gauge after 10 seconds
            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                // Remove the metric from Prometheus
                let _ = gauge_clone.remove_label_values(&[&id.to_string()]);
            });
        }
    });
}

async fn intercept_submit_share_success(builder: &mut ProxyBuilder, valid_shares: Counter) {
    let mut r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesSuccess(m))) = r.recv().await {
            println!("SubmitSharesSuccess received --> {:?}", m);
            valid_shares.inc();
        }
    });
}

async fn intercept_submit_share_error(builder: &mut ProxyBuilder, stale_shares: Counter) {
    let mut r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_ERROR);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesError(m))) = r.recv().await {
            println!("SubmitSharesError received --> {:?}", m);
            stale_shares.inc();
        }
    });
}

async fn intercept_submit_solution(
    builder: &mut ProxyBuilder,
    latency_metric: Gauge,
) {
    let mut r = builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SOLUTION);
    let client = Client::new(); 
    
    tokio::spawn(async move {
        while let Some(PoolMessages::TemplateDistribution(TemplateDistribution::SubmitSolution(m))) = r.recv().await {
            println!("SubmitSolution received --> {:?}", m);
            let id = m.header_nonce; 
            // Query the latency metric from Prometheus
            let url = format!("http://10.5.0.17:3456/metrics"); 
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(body) = response.text().await {
                    // Simple parsing to find the metric for the specific nonce
                    for line in body.lines() {
                        if line.starts_with("share_submission_timestamp{nonce=\"") && line.contains(&id.to_string()) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if let Some(timestamp_str) = parts.get(1) {
                                if let Ok(previous_timestamp) = timestamp_str.parse::<f64>() {
                                    // Get the current timestamp
                                    let current_timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                        .expect("Time went backwards")
                                        .as_millis() as f64;

                                    println!("Previous timestamp: {:?}", previous_timestamp);
                                    println!("Current timestamp: {:?}", current_timestamp);

                                    // Calculate latency
                                    let latency = current_timestamp - previous_timestamp;
                                    println!("Computed latency: {:?}", latency);

                                    // Set the latency metric
                                    latency_metric.set(latency);
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}