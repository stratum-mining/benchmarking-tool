mod proxy;

use stratum_common::roles_logic_sv2::{
    common_messages_sv2::{
        MESSAGE_TYPE_RECONNECT, MESSAGE_TYPE_SETUP_CONNECTION_ERROR,
    },
    job_declaration_sv2::{
        MESSAGE_TYPE_ALLOCATE_MINING_JOB_TOKEN, MESSAGE_TYPE_ALLOCATE_MINING_JOB_TOKEN_SUCCESS,
        MESSAGE_TYPE_DECLARE_MINING_JOB, MESSAGE_TYPE_DECLARE_MINING_JOB_ERROR,
        MESSAGE_TYPE_DECLARE_MINING_JOB_SUCCESS, MESSAGE_TYPE_PROVIDE_MISSING_TRANSACTIONS_SUCCESS,
    },
    mining_sv2::{
        MESSAGE_TYPE_CLOSE_CHANNEL, MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
        MESSAGE_TYPE_NEW_MINING_JOB, MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCESS,
        MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL_SUCCESS, MESSAGE_TYPE_SET_TARGET,
        MESSAGE_TYPE_SUBMIT_SHARES_ERROR, MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED,
        MESSAGE_TYPE_SUBMIT_SHARES_STANDARD, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS,
        MESSAGE_TYPE_UPDATE_CHANNEL,
    },
    parsers_sv2::{AnyMessage, CommonMessages, JobDeclaration, Mining, TemplateDistribution},
    template_distribution_sv2::{
        MESSAGE_TYPE_NEW_TEMPLATE, MESSAGE_TYPE_SET_NEW_PREV_HASH, MESSAGE_TYPE_SUBMIT_SOLUTION,
    },
};
use prometheus::{
    register_counter, register_gauge, register_gauge_vec, Counter, Encoder, Gauge, GaugeVec,
    TextEncoder,
};
use proxy::{ProxyBuilder, Remote};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::fmt::Write;
use std::net::ToSocketAddrs;
use std::time::SystemTime;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
use warp::Filter;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("SV2 Custom Proxy starting...");
    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");
    let proxy_type = env::var("PROXY_TYPE").expect("PROXY_TYPE environment variable not set");
    let prometheus_exporter_address =
        env::var("PROM_ADDRESS").expect("PROM_ADDRESS environment variable not set");

    let mut submitted_shares: Option<Counter> = None;
    let mut valid_shares: Option<Counter> = None;
    let mut stale_shares: Option<Counter> = None;
    let mut share_submission_timestamp: Option<GaugeVec> = None;
    let mut sv2_new_job_prev_hash_timestamp_jdc: Option<GaugeVec> = None;
    let mut sv2_new_job_prev_hash_timestamp_pool: Option<GaugeVec> = None;
    let mut sv2_new_job_timestamp_jdc: Option<GaugeVec> = None;
    let mut sv2_new_job_timestamp_pool: Option<GaugeVec> = None;
    let mut sv2_block_template_value: Option<Gauge> = None;
    let mut last_block_mined_value: Option<Gauge> = None;
    let mut last_sv2_block_template_value: Option<Gauge> = None;
    let mut block_propagation_time_through_sv2_jdc: Option<Gauge> = None;
    let mut block_propagation_time_through_sv2_pool: Option<Gauge> = None;
    let mut mined_blocks: Option<Counter> = None;

    // JDC-specific metrics
    let mut jdc_job_tokens_requested: Option<Counter> = None;
    let mut jdc_job_tokens_allocated: Option<Counter> = None;
    let mut jdc_job_token_allocation_latency: Option<GaugeVec> = None;
    let mut jdc_jobs_declared: Option<Counter> = None;
    let mut jdc_jobs_declared_success: Option<Counter> = None;
    let mut jdc_jobs_declared_error: Option<Counter> = None;
    let mut jdc_job_declaration_latency: Option<GaugeVec> = None;
    let mut jdc_custom_job_set_success: Option<Counter> = None;
    let mut jdc_custom_job_set_error: Option<Counter> = None;
    let mut jdc_missing_transactions_provided: Option<Counter> = None;

    // Mining job distribution metrics
    let mut mining_jobs_received: Option<Counter> = None;
    let mut extended_mining_jobs_received: Option<Counter> = None;
    let mut mining_job_latency: Option<GaugeVec> = None;
    let mut standard_shares_submitted: Option<Counter> = None;

    // Channel management metrics
    let mut channels_opened: Option<Counter> = None;
    let mut channel_open_latency: Option<GaugeVec> = None;
    let mut channels_closed: Option<Counter> = None;
    let mut channel_updates: Option<Counter> = None;
    let mut target_adjustments: Option<Counter> = None;
    let mut current_target: Option<Gauge> = None;

    // Connection health metrics
    let mut connection_errors: Option<Counter> = None;
    let mut reconnects: Option<Counter> = None;

    // Initialize metrics based on proxy_type
    match proxy_type.as_str() {
        "tp-jdc" => {
            mined_blocks = Some(
                register_counter!("sv2_mined_blocks", "Total number of SV2 blocks mined").unwrap(),
            );
            block_propagation_time_through_sv2_jdc = Some(
                register_gauge!(
                    "block_propagation_time_through_sv2_jdc",
                    "Time to submit a block through SV2 JDC in milliseconds"
                )
                .unwrap(),
            );
            sv2_new_job_prev_hash_timestamp_jdc = Some(
                register_gauge_vec!(
                    "sv2_new_job_prev_hash_timestamp_jdc",
                    "Time taken for mining device to get notification of new prev hash via config a",
                    &["prevhash"]
                ).unwrap()
            );
            sv2_new_job_timestamp_jdc = Some(
                register_gauge_vec!(
                    "sv2_new_job_timestamp_jdc",
                    "Time taken for mining device to get notification of new job via config a",
                    &["id"]
                )
                .unwrap(),
            );
            sv2_block_template_value = Some(
                register_gauge!(
                    "sv2_block_template_value",
                    "Total reward of sats contained in the current SV2 block template"
                )
                .unwrap(),
            );
            last_block_mined_value = Some(
                register_gauge!(
                    "last_block_mined_value",
                    "Total reward of sats contained in the last block mined"
                )
                .unwrap(),
            );
            last_sv2_block_template_value = Some(
                register_gauge!(
                    "last_sv2_template_value",
                    "Total reward of sats contained in the last SV2 block template"
                )
                .unwrap(),
            );
            // JDC-specific metrics
            jdc_job_tokens_requested = Some(
                register_counter!("jdc_job_tokens_requested", "Total JDC job tokens requested").unwrap(),
            );
            jdc_job_tokens_allocated = Some(
                register_counter!("jdc_job_tokens_allocated", "Total JDC job tokens allocated").unwrap(),
            );
            jdc_job_token_allocation_latency = Some(
                register_gauge_vec!(
                    "jdc_job_token_allocation_latency",
                    "Latency for JDC job token allocation in milliseconds",
                    &["token_id"]
                ).unwrap(),
            );
            jdc_jobs_declared = Some(
                register_counter!("jdc_jobs_declared", "Total JDC jobs declared").unwrap(),
            );
            jdc_jobs_declared_success = Some(
                register_counter!("jdc_jobs_declared_success", "Total JDC jobs declared successfully").unwrap(),
            );
            jdc_jobs_declared_error = Some(
                register_counter!("jdc_jobs_declared_error", "Total JDC job declaration errors").unwrap(),
            );
            jdc_job_declaration_latency = Some(
                register_gauge_vec!(
                    "jdc_job_declaration_latency",
                    "Latency for JDC job declaration in milliseconds",
                    &["request_id"]
                ).unwrap(),
            );
            jdc_custom_job_set_success = Some(
                register_counter!("jdc_custom_job_set_success", "Total custom jobs set successfully").unwrap(),
            );
            jdc_custom_job_set_error = Some(
                register_counter!("jdc_custom_job_set_error", "Total custom job set errors").unwrap(),
            );
            jdc_missing_transactions_provided = Some(
                register_counter!("jdc_missing_transactions_provided", "Total missing transactions provided").unwrap(),
            );
            // Connection health metrics
            connection_errors = Some(
                register_counter!("connection_errors", "Total connection errors").unwrap(),
            );
            reconnects = Some(
                register_counter!("reconnects", "Total reconnection attempts").unwrap(),
            );
        }
        "tp-pool" => {
            mined_blocks = Some(
                register_counter!("sv2_mined_blocks", "Total number of SV2 blocks mined").unwrap(),
            );
            block_propagation_time_through_sv2_pool = Some(
                register_gauge!(
                    "block_propagation_time_through_sv2_pool",
                    "Time to submit a block through SV2 Pool in milliseconds"
                )
                .unwrap(),
            );
            sv2_new_job_prev_hash_timestamp_pool = Some(
                register_gauge_vec!(
                    "sv2_new_job_prev_hash_timestamp_pool",
                    "Time taken for mining device to get notification of new prev via config c",
                    &["prevhash"]
                )
                .unwrap(),
            );
            sv2_new_job_timestamp_pool = Some(
                register_gauge_vec!(
                    "sv2_new_job_timestamp_pool",
                    "Time taken for mining device to get notification of new job via config c",
                    &["id"]
                )
                .unwrap(),
            );
            sv2_block_template_value = Some(
                register_gauge!(
                    "sv2_block_template_value",
                    "Total reward of sats contained in the current SV2 block template"
                )
                .unwrap(),
            );
            last_block_mined_value = Some(
                register_gauge!(
                    "last_block_mined_value",
                    "Total reward of sats contained in the last block mined"
                )
                .unwrap(),
            );
            last_sv2_block_template_value = Some(
                register_gauge!(
                    "last_sv2_template_value",
                    "Total reward of sats contained in the last SV2 block template"
                )
                .unwrap(),
            );
            // Connection health metrics
            connection_errors = Some(
                register_counter!("connection_errors", "Total connection errors").unwrap(),
            );
            reconnects = Some(
                register_counter!("reconnects", "Total reconnection attempts").unwrap(),
            );
        }
        "pool-translator" | "jdc-translator" => {
            submitted_shares = Some(
                register_counter!(
                    "sv2_submitted_shares",
                    "Total number of SV2 submitted shares"
                )
                .unwrap(),
            );
            valid_shares = Some(
                register_counter!("sv2_valid_shares", "Total number of SV2 valid shares").unwrap(),
            );
            stale_shares = Some(
                register_counter!("sv2_stale_shares", "Total number of SV2 stale shares").unwrap(),
            );
            share_submission_timestamp = Some(
                register_gauge_vec!(
                    "share_submission_timestamp",
                    "Timestamp of the submitted share",
                    &["nonce"]
                )
                .unwrap(),
            );
            standard_shares_submitted = Some(
                register_counter!("sv2_standard_shares_submitted", "Total standard shares submitted").unwrap(),
            );
            // Mining job distribution metrics
            mining_jobs_received = Some(
                register_counter!("mining_jobs_received", "Total mining jobs received").unwrap(),
            );
            extended_mining_jobs_received = Some(
                register_counter!("extended_mining_jobs_received", "Total extended mining jobs received").unwrap(),
            );
            mining_job_latency = Some(
                register_gauge_vec!(
                    "mining_job_latency",
                    "Latency for mining job distribution in milliseconds",
                    &["job_id"]
                ).unwrap(),
            );
            // Channel management metrics
            channels_opened = Some(
                register_counter!("channels_opened", "Total channels opened").unwrap(),
            );
            channel_open_latency = Some(
                register_gauge_vec!(
                    "channel_open_latency",
                    "Latency for channel opening in milliseconds",
                    &["channel_id"]
                ).unwrap(),
            );
            channels_closed = Some(
                register_counter!("channels_closed", "Total channels closed").unwrap(),
            );
            channel_updates = Some(
                register_counter!("channel_updates", "Total channel updates").unwrap(),
            );
            target_adjustments = Some(
                register_counter!("target_adjustments", "Total difficulty target adjustments").unwrap(),
            );
            current_target = Some(
                register_gauge!("current_target", "Current mining difficulty target").unwrap(),
            );
            // Connection health metrics
            connection_errors = Some(
                register_counter!("connection_errors", "Total connection errors").unwrap(),
            );
            reconnects = Some(
                register_counter!("reconnects", "Total reconnection attempts").unwrap(),
            );
        }
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
        let addr: std::net::SocketAddr = prometheus_exporter_address
            .parse()
            .expect("Invalid address");
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
            if let (Some(shares), Some(valid), Some(stale), Some(timestamp)) = (
                submitted_shares,
                valid_shares,
                stale_shares,
                share_submission_timestamp,
            ) {
                intercept_submit_share_extended(
                    &mut proxy_builder,
                    shares.clone(),
                    timestamp.clone(),
                )
                .await;
                intercept_submit_share_success(&mut proxy_builder, valid.clone()).await;
                intercept_submit_share_error(&mut proxy_builder, stale.clone()).await;
            }
            // Mining job distribution
            if let (Some(jobs), Some(latency)) = (mining_jobs_received, mining_job_latency) {
                intercept_new_mining_job(&mut proxy_builder, jobs.clone(), latency.clone()).await;
                intercept_new_extended_mining_job(&mut proxy_builder, extended_mining_jobs_received.clone().unwrap(), latency.clone()).await;
            }
            if let Some(standard) = standard_shares_submitted {
                intercept_submit_shares_standard(&mut proxy_builder, standard).await;
            }
            // Channel management
            if let (Some(opened), Some(latency)) = (channels_opened, channel_open_latency) {
                intercept_open_channel_success(&mut proxy_builder, opened.clone(), latency.clone(), MESSAGE_TYPE_OPEN_STANDARD_MINING_CHANNEL_SUCCESS).await;
                intercept_open_channel_success(&mut proxy_builder, opened.clone(), latency.clone(), MESSAGE_TYPE_OPEN_EXTENDED_MINING_CHANNEL_SUCCESS).await;
            }
            if let Some(closed) = channels_closed {
                intercept_close_channel(&mut proxy_builder, closed).await;
            }
            if let Some(updates) = channel_updates {
                intercept_update_channel(&mut proxy_builder, updates).await;
            }
            if let (Some(adjustments), Some(target)) = (target_adjustments, current_target) {
                intercept_set_target(&mut proxy_builder, adjustments, target).await;
            }
            // Connection health
            if let Some(errors) = connection_errors {
                intercept_setup_connection_error(&mut proxy_builder, errors).await;
            }
            if let Some(reconnect_counter) = reconnects {
                intercept_reconnect(&mut proxy_builder, reconnect_counter).await;
            }
        }
        "tp-pool" => {
            if let (
                Some(new_job_gauge_vec),
                Some(last_block_mined_value),
                Some(last_sv2_block_template_value),
            ) = (
                sv2_new_job_prev_hash_timestamp_pool,
                last_block_mined_value,
                last_sv2_block_template_value,
            ) {
                intercept_prev_hash(
                    &mut proxy_builder,
                    new_job_gauge_vec,
                    last_block_mined_value,
                    last_sv2_block_template_value,
                )
                .await;
            }
            if let (Some(pool_latency), Some(mined)) =
                (block_propagation_time_through_sv2_pool, mined_blocks)
            {
                intercept_submit_solution(&mut proxy_builder, pool_latency, mined).await;
            }
            if let (Some(new_job_pool), Some(sv2_block_template_value)) =
                (sv2_new_job_timestamp_pool, sv2_block_template_value)
            {
                intercept_new_template(&mut proxy_builder, new_job_pool, sv2_block_template_value)
                    .await;
            }
            // Connection health
            if let Some(errors) = connection_errors {
                intercept_setup_connection_error(&mut proxy_builder, errors).await;
            }
            if let Some(reconnect_counter) = reconnects {
                intercept_reconnect(&mut proxy_builder, reconnect_counter).await;
            }
        }
        "tp-jdc" => {
            if let (
                Some(new_job_gauge_vec),
                Some(last_block_mined_value),
                Some(last_sv2_block_template_value),
            ) = (
                sv2_new_job_prev_hash_timestamp_jdc,
                last_block_mined_value,
                last_sv2_block_template_value,
            ) {
                intercept_prev_hash(
                    &mut proxy_builder,
                    new_job_gauge_vec,
                    last_block_mined_value,
                    last_sv2_block_template_value,
                )
                .await;
            }
            if let (Some(jdc_latency), Some(mined)) =
                (block_propagation_time_through_sv2_jdc, mined_blocks)
            {
                intercept_submit_solution(&mut proxy_builder, jdc_latency, mined).await;
            }
            if let (Some(new_job_jdc), Some(sv2_block_template_value)) =
                (sv2_new_job_timestamp_jdc, sv2_block_template_value)
            {
                intercept_new_template(&mut proxy_builder, new_job_jdc, sv2_block_template_value)
                    .await;
            }
            // JDC-specific interceptors
            if let (Some(tokens_req), Some(latency)) = (jdc_job_tokens_requested, jdc_job_token_allocation_latency) {
                intercept_allocate_mining_job_token(&mut proxy_builder, tokens_req.clone(), latency.clone()).await;
                if let Some(tokens_alloc) = jdc_job_tokens_allocated {
                    intercept_allocate_mining_job_token_success(&mut proxy_builder, tokens_alloc, latency.clone()).await;
                }
            }
            if let (Some(declared), Some(latency)) = (jdc_jobs_declared, jdc_job_declaration_latency) {
                intercept_declare_mining_job(&mut proxy_builder, declared.clone(), latency.clone()).await;
                if let Some(success) = jdc_jobs_declared_success {
                    intercept_declare_mining_job_success(&mut proxy_builder, success, latency.clone()).await;
                }
            }
            if let Some(error) = jdc_jobs_declared_error {
                intercept_declare_mining_job_error(&mut proxy_builder, error).await;
            }
            if let Some(missing_tx) = jdc_missing_transactions_provided {
                intercept_provide_missing_transactions_success(&mut proxy_builder, missing_tx).await;
            }
            // Connection health
            if let Some(errors) = connection_errors {
                intercept_setup_connection_error(&mut proxy_builder, errors).await;
            }
            if let Some(reconnect_counter) = reconnects {
                intercept_reconnect(&mut proxy_builder, reconnect_counter).await;
            }
        }
        _ => {
            panic!("Invalid PROXY_TYPE");
        }
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

    let max_retries = 10;
    let mut retry_count = 0;

    loop {
        match TcpStream::connect(address).await {
            Ok(stream) => {
                info!("Successfully connected to server at {}", server_address);
                return stream;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= max_retries {
                    error!("Failed to connect to server at {} after {} attempts: {}",
                               server_address, max_retries, e);
                    panic!("Unable to connect to server: {}", e);
                }
                warn!("Failed to connect to server at {} (attempt {}/{}): {}. Retrying in 2s...",
                          server_address, retry_count, max_retries, e);
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

pub fn encode_hex(bytes: &[u8]) -> String {
    let hex_string = bytes.iter().fold(String::new(), |mut acc, b| {
        write!(&mut acc, "{:02x}", b).unwrap();
        acc
    });
    hex_string
}

fn reverse_hash(hash: &str) -> String {
    hash.as_bytes()
        .chunks(2)
        .map(|chunk| {
            let hex_str = std::str::from_utf8(chunk).expect("Invalid UTF-8 sequence");
            u8::from_str_radix(hex_str, 16).expect("Invalid hex number")
        })
        .rev()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}

async fn fetch_block_reward(hash: &str) -> Result<u64, String> {
    let network = env::var("NETWORK").unwrap_or_else(|_| "".to_string());
    let client = Client::new();
    let url = match network.as_str() {
        "" => format!("https://mempool.space/api/block/{}", hash),
        "testnet3" => format!("https://mempool.space/testnet/api/block/{}", hash),
        "testnet4" => format!("https://mempool.space/testnet4/api/block/{}", hash),
        _ => return Err("Invalid NETWORK environment variable".to_string()),
    };

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let body = response.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let height = json["height"]
        .as_u64()
        .ok_or("Failed to parse height from response")?;

    let reward_stats_url = match network.as_str() {
        "" => "https://mempool.space/api/v1/mining/reward-stats/1",
        "testnet3" => "https://mempool.space/testnet/api/v1/mining/reward-stats/1",
        "testnet4" => "https://mempool.space/testnet4/api/v1/mining/reward-stats/1",
        _ => unreachable!(),
    };

    let reward_stats_response = client
        .get(reward_stats_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let reward_stats_body = reward_stats_response
        .text()
        .await
        .map_err(|e| e.to_string())?;
    let reward_stats_json: Value =
        serde_json::from_str(&reward_stats_body).map_err(|e| e.to_string())?;

    let start_block = reward_stats_json["startBlock"]
        .as_u64()
        .ok_or("Failed to parse startBlock from reward stats")?;
    let end_block = reward_stats_json["endBlock"]
        .as_u64()
        .ok_or("Failed to parse endBlock from reward stats")?;

    if start_block == end_block && end_block == height {
        let total_reward = reward_stats_json["totalReward"]
            .as_str()
            .ok_or("Failed to parse totalReward from reward stats")?;
        total_reward.parse::<u64>().map_err(|e| e.to_string())
    } else {
        Err("Block height mismatch".to_string())
    }
}

async fn fetch_last_block_reward_with_retries(
    hash: &str,
    retries: usize,
    delay: Duration,
) -> Result<u64, String> {
    let mut attempt = 0;
    while attempt < retries {
        match fetch_block_reward(hash).await {
            Ok(reward) => return Ok(reward),
            Err(e) => {
                error!("Attempt {} failed: {}", attempt + 1, e);
                attempt += 1;
                sleep(delay).await;
            }
        }
    }
    Err("Failed to fetch block reward after multiple attempts".to_string())
}

async fn fetch_metric_from_prometheus(
    prometheus_address: &str,
    metric_name: &str,
    timestamp: f64,
) -> Result<f64, String> {
    let client = Client::new();
    let url = format!(
        "{}/api/v1/query?query={} &time={}",
        prometheus_address, metric_name, timestamp
    );
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let body = response.text().await.map_err(|e| e.to_string())?;
    let json: Value =
        serde_json::from_str(&body).map_err(|e| format!("Error parsing JSON: {}", e))?;

    // Parse the result
    let value = json["data"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|obj| obj["value"].as_array())
        .and_then(|values| values.get(1))
        .and_then(|val_str| val_str.as_str())
        .and_then(|val_str| val_str.parse::<f64>().ok())
        .ok_or("Failed to parse metric value".to_string())?;

    Ok(value)
}

async fn intercept_prev_hash(
    builder: &mut ProxyBuilder,
    new_prev_hash_timestamp: GaugeVec,
    last_block_mined_value: Gauge,
    last_sv2_block_template_value: Gauge,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SET_NEW_PREV_HASH);
    tokio::spawn(async move {
        while let Ok(AnyMessage::TemplateDistribution(TemplateDistribution::SetNewPrevHash(m))) =
            r.recv().await
        {
            let mut id = m.prev_hash;
            let d = id.inner_as_mut();
            let prev_hash_hex = encode_hex(d);
            let new_prev_hash_timestamp_clone = new_prev_hash_timestamp.clone();
            let last_block_mined_value_clone = last_block_mined_value.clone();
            let last_sv2_block_template_value_clone = last_sv2_block_template_value.clone();
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            new_prev_hash_timestamp_clone
                .with_label_values(&[&prev_hash_hex])
                .set(current_time);
            tokio::spawn(async move {
                sleep(Duration::from_secs(1)).await;
                // Remove the metric from Prometheus
                let _ = new_prev_hash_timestamp_clone.remove_label_values(&[&prev_hash_hex]);

                let now = SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs_f64();
                let timestamp = now - 2.0;
                // Fetch the previous template value from Prometheus
                let fetch_metric_result = fetch_metric_from_prometheus(
                    "http://10.5.0.9:9090",
                    "sv2_block_template_value",
                    timestamp,
                )
                .await;

                // Fetch the last block reward and set last_block_mined_value
                if let Ok(reward) = fetch_last_block_reward_with_retries(
                    &reverse_hash(&prev_hash_hex),
                    24,
                    Duration::from_secs(5),
                )
                .await
                {
                    let reward_as_f64 = reward as f64;
                    last_block_mined_value_clone.set(reward_as_f64);

                    // Set the fetched metric value for last_sv2_block_template_value
                    if let Ok(value) = fetch_metric_result {
                        last_sv2_block_template_value_clone.set(value);
                    } else {
                        error!("Error fetching metric");
                    }
                }
            });
        }
    });
}

async fn intercept_new_template(
    builder: &mut ProxyBuilder,
    new_job_timestamp: GaugeVec,
    sv2_block_template_value: Gauge,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_NEW_TEMPLATE);
    tokio::spawn(async move {
        while let Ok(AnyMessage::TemplateDistribution(TemplateDistribution::NewTemplate(m))) =
            r.recv().await
        {
            let id = m.template_id;
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let new_job_timestamp_clone = new_job_timestamp.clone();
            new_job_timestamp_clone
                .with_label_values(&[&id.to_string()])
                .set(current_time);
            tokio::spawn(async move {
                sleep(Duration::from_secs(1)).await;
                // Remove the metric from Prometheus
                let _ = new_job_timestamp_clone.remove_label_values(&[&id.to_string()]);
            });
            // Take the coinbase value and set the block template value metric
            let sv2_block_template_value_clone = sv2_block_template_value.clone();
            let block_value = m.coinbase_tx_value_remaining;
            sv2_block_template_value_clone.set(block_value as f64);
        }
    });
}

async fn intercept_submit_share_extended(
    builder: &mut ProxyBuilder,
    submitted_shares: Counter,
    gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::SubmitSharesExtended(m))) = r.recv().await {
            submitted_shares.inc();

            let id = m.nonce;
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;

            let gauge_clone = gauge.clone();
            gauge_clone
                .with_label_values(&[&id.to_string()])
                .set(current_time);

            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                // Remove the metric from Prometheus
                let _ = gauge_clone.remove_label_values(&[&id.to_string()]);
            });
        }
    });
}

async fn intercept_submit_share_success(builder: &mut ProxyBuilder, valid_shares: Counter) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::SubmitSharesSuccess(_m))) = r.recv().await {
            valid_shares.inc();
        }
    });
}

async fn intercept_submit_share_error(builder: &mut ProxyBuilder, stale_shares: Counter) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_ERROR);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::SubmitSharesError(m))) = r.recv().await {
            error!("SubmitSharesError received --> {:?}", m);
            stale_shares.inc();
        }
    });
}

async fn intercept_submit_solution(
    builder: &mut ProxyBuilder,
    block_propagation_time: Gauge,
    mined_blocks: Counter,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SOLUTION);
    let client = Client::new();

    tokio::spawn(async move {
        while let Ok(AnyMessage::TemplateDistribution(TemplateDistribution::SubmitSolution(m))) =
            r.recv().await
        {
            let current_timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let id = m.header_nonce;
            let url = "http://10.5.0.17:3456/metrics".to_string();
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(body) = response.text().await {
                    // Simple parsing to find the metric for the specific nonce
                    for line in body.lines() {
                        if line.starts_with("share_submission_timestamp{nonce=\"")
                            && line.contains(&id.to_string())
                        {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if let Some(timestamp_str) = parts.get(1) {
                                if let Ok(previous_timestamp) = timestamp_str.parse::<f64>() {
                                    let latency = current_timestamp - previous_timestamp;
                                    block_propagation_time.set(latency);
                                    mined_blocks.inc();
                                }
                            }
                        }
                    }
                }
            }
        }
    });
}

// JDC job declaration interceptors
async fn intercept_allocate_mining_job_token(
    builder: &mut ProxyBuilder,
    tokens_requested: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_ALLOCATE_MINING_JOB_TOKEN);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::AllocateMiningJobToken(m))) =
            r.recv().await
        {
            tokens_requested.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let token_id = String::from_utf8_lossy(m.user_identifier.inner_as_ref()).to_string();
            latency_gauge.with_label_values(&[&token_id]).set(current_time);

            let gauge_clone = latency_gauge.clone();
            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                let _ = gauge_clone.remove_label_values(&[&token_id]);
            });
        }
    });
}

async fn intercept_allocate_mining_job_token_success(
    builder: &mut ProxyBuilder,
    tokens_allocated: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_ALLOCATE_MINING_JOB_TOKEN_SUCCESS);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::AllocateMiningJobTokenSuccess(m))) =
            r.recv().await
        {
            tokens_allocated.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;

            // Calculate latency from request
            let token_id = m.mining_job_token.to_vec().iter().map(|b| format!("{:02x}", b)).collect::<String>();
            let request_time = latency_gauge.with_label_values(&[&token_id]).get();
            let latency = current_time - request_time;
            if latency > 0.0 {
                info!("JDC job token allocated in {} ms", latency);
            }
        }
    });
}

async fn intercept_declare_mining_job(
    builder: &mut ProxyBuilder,
    jobs_declared: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_DECLARE_MINING_JOB);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::DeclareMiningJob(m))) =
            r.recv().await
        {
            jobs_declared.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let request_id = format!("{}", m.request_id);
            latency_gauge.with_label_values(&[&request_id]).set(current_time);

            let gauge_clone = latency_gauge.clone();
            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                let _ = gauge_clone.remove_label_values(&[&request_id]);
            });
        }
    });
}

async fn intercept_declare_mining_job_success(
    builder: &mut ProxyBuilder,
    jobs_success: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_DECLARE_MINING_JOB_SUCCESS);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::DeclareMiningJobSuccess(m))) =
            r.recv().await
        {
            jobs_success.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;

            let request_id = format!("{}", m.request_id);
            let request_time = latency_gauge.with_label_values(&[&request_id]).get();
            let latency = current_time - request_time;
            if latency > 0.0 {
                info!("JDC job declared successfully in {} ms", latency);
            }
        }
    });
}

async fn intercept_declare_mining_job_error(
    builder: &mut ProxyBuilder,
    jobs_error: Counter,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_DECLARE_MINING_JOB_ERROR);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::DeclareMiningJobError(m))) =
            r.recv().await
        {
            error!("JDC job declaration error: {:?}", m.error_code.as_ref());
            jobs_error.inc();
        }
    });
}

async fn intercept_provide_missing_transactions_success(
    builder: &mut ProxyBuilder,
    counter: Counter,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_PROVIDE_MISSING_TRANSACTIONS_SUCCESS);
    tokio::spawn(async move {
        while let Ok(AnyMessage::JobDeclaration(JobDeclaration::ProvideMissingTransactionsSuccess(_m))) =
            r.recv().await
        {
            counter.inc();
        }
    });
}

// Mining job distribution interceptors
async fn intercept_new_mining_job(
    builder: &mut ProxyBuilder,
    jobs_received: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_NEW_MINING_JOB);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::NewMiningJob(m))) = r.recv().await {
            jobs_received.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let job_id = format!("{}", m.job_id);
            latency_gauge.with_label_values(&[&job_id]).set(current_time);

            let gauge_clone = latency_gauge.clone();
            tokio::spawn(async move {
                sleep(Duration::from_secs(5)).await;
                let _ = gauge_clone.remove_label_values(&[&job_id]);
            });
        }
    });
}

async fn intercept_new_extended_mining_job(
    builder: &mut ProxyBuilder,
    jobs_received: Counter,
    latency_gauge: GaugeVec,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::NewExtendedMiningJob(m))) = r.recv().await {
            jobs_received.inc();
            let current_time = SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as f64;
            let job_id = format!("{}", m.job_id);
            latency_gauge.with_label_values(&[&job_id]).set(current_time);

            let gauge_clone = latency_gauge.clone();
            tokio::spawn(async move {
                sleep(Duration::from_secs(5)).await;
                let _ = gauge_clone.remove_label_values(&[&job_id]);
            });
        }
    });
}

async fn intercept_submit_shares_standard(
    builder: &mut ProxyBuilder,
    standard_shares: Counter,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SHARES_STANDARD);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::SubmitSharesStandard(_m))) = r.recv().await {
            standard_shares.inc();
        }
    });
}

// Channel management interceptors
async fn intercept_open_channel_success(
    builder: &mut ProxyBuilder,
    channels_opened: Counter,
    latency_gauge: GaugeVec,
    message_type: u8,
) {
    let r = builder.add_handler(Remote::Server, message_type);
    tokio::spawn(async move {
        loop {
            let msg = r.recv().await;
            match msg {
                Ok(AnyMessage::Mining(Mining::OpenStandardMiningChannelSuccess(m))) => {
                    channels_opened.inc();
                    let current_time = SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as f64;
                    let channel_id = format!("{}", m.channel_id);
                    latency_gauge.with_label_values(&[&channel_id]).set(current_time);
                }
                Ok(AnyMessage::Mining(Mining::OpenExtendedMiningChannelSuccess(m))) => {
                    channels_opened.inc();
                    let current_time = SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as f64;
                    let channel_id = format!("{}", m.channel_id);
                    latency_gauge.with_label_values(&[&channel_id]).set(current_time);
                }
                _ => {}
            }
        }
    });
}

async fn intercept_close_channel(
    builder: &mut ProxyBuilder,
    channels_closed: Counter,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_CLOSE_CHANNEL);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::CloseChannel(m))) = r.recv().await {
            info!("Channel {} closed: {}", m.channel_id, String::from_utf8_lossy(m.reason_code.as_ref()));
            channels_closed.inc();
        }
    });
}

async fn intercept_update_channel(
    builder: &mut ProxyBuilder,
    channel_updates: Counter,
) {
    let r = builder.add_handler(Remote::Client, MESSAGE_TYPE_UPDATE_CHANNEL);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::UpdateChannel(_m))) = r.recv().await {
            channel_updates.inc();
        }
    });
}

async fn intercept_set_target(
    builder: &mut ProxyBuilder,
    target_adjustments: Counter,
    current_target: Gauge,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SET_TARGET);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Mining(Mining::SetTarget(m))) = r.recv().await {
            target_adjustments.inc();
            // Convert target bytes to a numeric representation
            let target_bytes = m.maximum_target.inner_as_ref();
            let target_value = u32::from_le_bytes([
                target_bytes[0],
                target_bytes[1],
                target_bytes[2],
                target_bytes[3],
            ]) as f64;
            current_target.set(target_value);
            info!("Difficulty target adjusted to {}", target_value);
        }
    });
}

// Connection health interceptors
async fn intercept_setup_connection_error(
    builder: &mut ProxyBuilder,
    connection_errors: Counter,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_SETUP_CONNECTION_ERROR);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Common(CommonMessages::SetupConnectionError(m))) = r.recv().await {
            error!("Setup connection error: {}", String::from_utf8_lossy(m.error_code.as_ref()));
            connection_errors.inc();
        }
    });
}

async fn intercept_reconnect(
    builder: &mut ProxyBuilder,
    reconnects: Counter,
) {
    let r = builder.add_handler(Remote::Server, MESSAGE_TYPE_RECONNECT);
    tokio::spawn(async move {
        while let Ok(AnyMessage::Common(CommonMessages::Reconnect(_m))) = r.recv().await {
            warn!("Reconnect requested by server");
            reconnects.inc();
        }
    });
}
