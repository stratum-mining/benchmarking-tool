use prometheus::{
    register_counter, register_gauge, register_gauge_vec, Counter, Encoder, Gauge, GaugeVec,
    TextEncoder,
};
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::mpsc::channel,
    time::{sleep, Duration},
};
use warp::Filter;

// Import from stratum-apps
use stratum_apps::{
    key_utils::{Secp256k1PublicKey, Secp256k1SecretKey},
    network_helpers::noise_stream::NoiseTcpStream,
    stratum_core::{
        binary_sv2::Deserialize,
        codec_sv2::HandshakeRole,
        framing_sv2::framing::Frame,
        mining_sv2::{
            NewExtendedMiningJob, SubmitSharesExtended, MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB,
            MESSAGE_TYPE_SUBMIT_SHARES_ERROR, MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED,
            MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS,
        },
        noise_sv2::{Initiator, Responder},
        template_distribution_sv2::{
            NewTemplate, SetNewPrevHash, SubmitSolution, MESSAGE_TYPE_NEW_TEMPLATE,
            MESSAGE_TYPE_SET_NEW_PREV_HASH, MESSAGE_TYPE_SUBMIT_SOLUTION,
        },
    },
    utils::types::{Message, Sv2Frame},
};

// Default authority keys (same as in demand-easy-sv2)
const DEFAULT_PUBKEY: &str = "9auqWEzQDVyd2oe1JVGFLMLHZtCo2FFqZwtKA5gd9xbuEu7PH72";
const DEFAULT_SECKEY: &str = "mkDLTBBRxdBv998612qipDYoTK3YUrqLe8uWw7gu3iXbSrn2n";

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut acc, b| {
        write!(acc, "{:02x}", b).unwrap();
        acc
    })
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
                log::error!("Attempt {} failed: {}", attempt + 1, e);
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
        "{}/api/v1/query?query={}&time={}",
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

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default()
            .default_filter_or("info")
            .default_write_style_or("always"),
    )
    .is_test(true)
    .init();

    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");
    let proxy_type = env::var("PROXY_TYPE").expect("PROXY_TYPE environment variable not set");
    let prometheus_address =
        env::var("PROM_ADDRESS").expect("PROM_ADDRESS environment variable not set");

    log::info!("Starting SV2 transparent proxy");
    log::info!("  Type: {}", proxy_type);
    log::info!("  Listen: {}", client_address);
    log::info!("  Connect: {}", server_address);
    log::info!("  Metrics: {}", prometheus_address);

    // Initialize metrics based on proxy type
    let metrics = Arc::new(Metrics::new(&proxy_type));

    // Start Prometheus exporter
    let prom_addr: SocketAddr = prometheus_address
        .parse()
        .expect("Invalid prometheus address");
    tokio::spawn(async move {
        let metrics_route = warp::path!("metrics").map(move || {
            let encoder = TextEncoder::new();
            let metric_families = prometheus::gather();
            let mut buffer = vec![];
            encoder.encode(&metric_families, &mut buffer).unwrap();
            String::from_utf8(buffer).unwrap()
        });
        log::info!("Prometheus exporter listening on http://{}", prom_addr);
        warp::serve(metrics_route).run(prom_addr).await;
    });

    // Start background tasks for specific proxy types
    if proxy_type == "tp-jdc" || proxy_type == "tp-pool" {
        let metrics_clone = metrics.clone();
        tokio::spawn(async move {
            fetch_block_template_value(metrics_clone).await;
        });
    }

    // Start the proxy server
    let listener = TcpListener::bind(&client_address)
        .await
        .expect("Failed to bind to client address");
    log::info!("Proxy listening on {}", client_address);

    loop {
        match listener.accept().await {
            Ok((client_stream, client_addr)) => {
                log::info!("New client connection from {}", client_addr);
                let server_addr = server_address.clone();
                let proxy_type = proxy_type.clone();
                let metrics = metrics.clone();

                tokio::spawn(async move {
                    if let Err(e) =
                        handle_connection(client_stream, &server_addr, &proxy_type, metrics).await
                    {
                        log::error!("Connection error: {:?}", e);
                    }
                });
            }
            Err(e) => {
                log::error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_connection(
    client_stream: TcpStream,
    server_address: &str,
    proxy_type: &str,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to upstream server
    let server_stream = TcpStream::connect(server_address).await?;
    log::info!("Connected to upstream server at {}", server_address);

    // Perform Noise handshakes
    // As responder for client (we are the server from client's perspective)
    let authority_public_key: Secp256k1PublicKey = DEFAULT_PUBKEY
        .parse()
        .map_err(|e| format!("Failed to parse pubkey: {:?}", e))?;
    let authority_secret_key: Secp256k1SecretKey = DEFAULT_SECKEY
        .parse()
        .map_err(|e| format!("Failed to parse seckey: {:?}", e))?;

    let responder = Responder::from_authority_kp(
        &authority_public_key.into_bytes(),
        &authority_secret_key.into_bytes(),
        std::time::Duration::from_secs(10000),
    )
    .map_err(|e| format!("Failed to create responder: {:?}", e))?;

    let client_noise_stream =
        NoiseTcpStream::<Message>::new(client_stream, HandshakeRole::Responder(responder))
            .await
            .map_err(|e| format!("Client Noise handshake failed: {:?}", e))?;
    log::info!("Client Noise handshake completed");

    // As initiator for server (we are the client from server's perspective)
    let initiator =
        Initiator::without_pk().map_err(|e| format!("Failed to create initiator: {:?}", e))?;
    let server_noise_stream =
        NoiseTcpStream::<Message>::new(server_stream, HandshakeRole::Initiator(initiator))
            .await
            .map_err(|e| format!("Server Noise handshake failed: {:?}", e))?;
    log::info!("Server Noise handshake completed");

    // Split the Noise streams
    let (mut client_reader, mut client_writer) = client_noise_stream.into_split();
    let (mut server_reader, mut server_writer) = server_noise_stream.into_split();

    // Create channels for bidirectional communication
    let (client_to_server_tx, mut client_to_server_rx) = channel::<Sv2Frame>(100);
    let (server_to_client_tx, mut server_to_client_rx) = channel::<Sv2Frame>(100);

    // Spawn task to read from client and forward to server
    let proxy_type_clone = proxy_type.to_string();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            match client_reader.read_frame().await {
                Ok(frame) => {
                    match frame {
                        Frame::HandShake(_) => {
                            log::warn!("Unexpected handshake frame from client");
                            break;
                        }
                        Frame::Sv2(mut sv2_frame) => {
                            // Intercept specific message types
                            if let Some(header) = sv2_frame.get_header() {
                                let msg_type = header.msg_type();

                                // Parse the frame if it needs payload parsing
                                if msg_type == MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED
                                    || msg_type == MESSAGE_TYPE_SUBMIT_SOLUTION
                                {
                                    let payload = sv2_frame.payload().to_vec(); // Clone the payload
                                    intercept_client_message_with_payload(
                                        msg_type,
                                        payload,
                                        &proxy_type_clone,
                                        &metrics_clone,
                                    )
                                    .await;
                                } else {
                                    intercept_client_message(
                                        msg_type,
                                        &proxy_type_clone,
                                        &metrics_clone,
                                    )
                                    .await;
                                }
                            }

                            // Forward to server
                            if client_to_server_tx.send(sv2_frame).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading from client: {:?}", e);
                    break;
                }
            }
        }
    });

    // Spawn task to read from server and forward to client
    let proxy_type_clone = proxy_type.to_string();
    let metrics_clone = metrics.clone();
    tokio::spawn(async move {
        loop {
            match server_reader.read_frame().await {
                Ok(frame) => {
                    match frame {
                        Frame::HandShake(_) => {
                            log::warn!("Unexpected handshake frame from server");
                            break;
                        }
                        Frame::Sv2(mut sv2_frame) => {
                            // Intercept specific message types
                            if let Some(header) = sv2_frame.get_header() {
                                let msg_type = header.msg_type();

                                // Parse the frame if it needs payload parsing
                                if msg_type == MESSAGE_TYPE_NEW_TEMPLATE
                                    || msg_type == MESSAGE_TYPE_SET_NEW_PREV_HASH
                                    || msg_type == MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB
                                {
                                    let payload = sv2_frame.payload().to_vec(); // Clone the payload
                                    intercept_server_message_with_payload(
                                        msg_type,
                                        payload,
                                        &proxy_type_clone,
                                        &metrics_clone,
                                    )
                                    .await;
                                } else {
                                    intercept_server_message(
                                        msg_type,
                                        &proxy_type_clone,
                                        &metrics_clone,
                                    )
                                    .await;
                                }
                            }

                            // Forward to client
                            if server_to_client_tx.send(sv2_frame).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading from server: {:?}", e);
                    break;
                }
            }
        }
    });

    // Forward messages between channels and streams
    loop {
        select! {
            Some(sv2_frame) = client_to_server_rx.recv() => {
                if let Err(e) = server_writer.write_frame(Frame::Sv2(sv2_frame)).await {
                    log::error!("Error writing to server: {:?}", e);
                    break;
                }
            }
            Some(sv2_frame) = server_to_client_rx.recv() => {
                if let Err(e) = client_writer.write_frame(Frame::Sv2(sv2_frame)).await {
                    log::error!("Error writing to client: {:?}", e);
                    break;
                }
            }
            else => break,
        }
    }

    log::info!("Connection closed");
    Ok(())
}

async fn intercept_client_message_with_payload(
    msg_type: u8,
    mut payload: Vec<u8>,
    proxy_type: &str,
    metrics: &Arc<Metrics>,
) {
    match msg_type {
        MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED => {
            log::debug!("Intercepted SubmitSharesExtended from client");
            // Try to parse the message to extract nonce
            match SubmitSharesExtended::from_bytes(&mut payload) {
                Ok(msg) => {
                    let nonce = msg.nonce;
                    log::info!("SubmitSharesExtended: nonce={}", nonce);

                    if let Some(counter) = &metrics.submitted_shares {
                        counter.inc();
                        log::info!(
                            "Metric recorded: sv2_submitted_shares incremented (nonce={})",
                            nonce
                        );
                    }

                    // Track timestamp
                    if let Some(gauge_vec) = &metrics.share_submission_timestamp {
                        let current_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_millis() as f64;

                        let nonce_str = nonce.to_string();
                        gauge_vec.with_label_values(&[&nonce_str]).set(current_time);
                        log::info!("Metric recorded: sv2_share_submission_timestamp set (nonce={}, timestamp={})", nonce, current_time);

                        // Schedule cleanup
                        let gauge_clone = gauge_vec.clone();
                        let nonce_clone = nonce_str.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(10)).await;
                            let _ = gauge_clone.remove_label_values(&[&nonce_clone]);
                        });
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse SubmitSharesExtended: {:?}", e);
                    // Still increment counter even if parsing fails
                    if let Some(counter) = &metrics.submitted_shares {
                        counter.inc();
                        log::info!(
                            "Metric recorded: sv2_submitted_shares incremented (parsing failed)"
                        );
                    }
                }
            }
        }
        MESSAGE_TYPE_SUBMIT_SOLUTION => {
            log::debug!("Intercepted SubmitSolution from client");
            // Try to parse the message to extract nonce
            // Note: This is received by tp-jdc or tp-pool sniffer
            // The share submission timestamp should have been recorded by jdc-translator or pool-translator sniffer
            match SubmitSolution::from_bytes(&mut payload) {
                Ok(msg) => {
                    let nonce = msg.header_nonce;
                    log::info!(
                        "SubmitSolution: header_nonce={}, template_id={}",
                        nonce,
                        msg.template_id
                    );

                    // Calculate block propagation time by looking up share submission timestamp
                    // First timestamp: recorded by jdc-translator or pool-translator sniffer when share was submitted
                    // Second timestamp: now (when tp-jdc or tp-pool sniffer receives SubmitSolution)
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as f64;

                    // Query Prometheus to find the share submission timestamp for this nonce
                    // This timestamp was recorded by jdc-translator or pool-translator sniffer
                    let prometheus_address = "http://10.5.0.50:9090";
                    let nonce_str = nonce.to_string();
                    let metric_name =
                        format!("sv2_share_submission_timestamp{{nonce=\"{}\"}}", nonce_str);
                    log::info!(
                        "Looking up share submission timestamp for nonce={} from Prometheus",
                        nonce_str
                    );

                    // Clone metrics for async task based on proxy type
                    let block_propagation_jdc =
                        metrics.block_propagation_time_through_sv2_jdc.clone();
                    let block_propagation_pool =
                        metrics.block_propagation_time_through_sv2_pool.clone();
                    let mined_blocks = metrics.mined_blocks.clone();
                    let proxy_type_str = proxy_type.to_string();

                    tokio::spawn(async move {
                        // Try to fetch the share submission timestamp from Prometheus
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_secs_f64();

                        // Try querying for the metric
                        match fetch_metric_from_prometheus(prometheus_address, &metric_name, now)
                            .await
                        {
                            Ok(share_timestamp) => {
                                if share_timestamp > 0.0 {
                                    let latency = current_time - share_timestamp;
                                    log::info!(
                                        "Block propagation time calculated: {}ms (nonce={}, share_timestamp={}, block_timestamp={})",
                                        latency,
                                        nonce_str,
                                        share_timestamp,
                                        current_time
                                    );

                                    // Set the appropriate metric based on proxy type
                                    match proxy_type_str.as_str() {
                                        "tp-jdc" => {
                                            if let Some(gauge) = &block_propagation_jdc {
                                                gauge.set(latency);
                                                log::info!(
                                                    "Metric recorded: block_propagation_time_through_sv2_jdc set (nonce={}, latency={}ms)",
                                                    nonce_str,
                                                    latency
                                                );
                                            }
                                            if let Some(counter) = &mined_blocks {
                                                counter.inc();
                                                log::info!("Metric recorded: sv2_mined_blocks incremented (nonce={})", nonce_str);
                                            }
                                        }
                                        "tp-pool" => {
                                            if let Some(gauge) = &block_propagation_pool {
                                                gauge.set(latency);
                                                log::info!(
                                                    "Metric recorded: block_propagation_time_through_sv2_pool set (nonce={}, latency={}ms)",
                                                    nonce_str,
                                                    latency
                                                );
                                            }
                                            if let Some(counter) = &mined_blocks {
                                                counter.inc();
                                                log::info!("Metric recorded: sv2_mined_blocks incremented (nonce={})", nonce_str);
                                            }
                                        }
                                        _ => {
                                            log::warn!(
                                                "Block propagation time metrics not registered for proxy type: {}",
                                                proxy_type_str
                                            );
                                        }
                                    }
                                } else {
                                    log::warn!(
                                        "Share submission timestamp not found for nonce={} (metric returned 0)",
                                        nonce_str
                                    );
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to fetch share submission timestamp for nonce={}: {}",
                                    nonce_str,
                                    e
                                );
                            }
                        }
                    });
                }
                Err(e) => {
                    log::warn!("Failed to parse SubmitSolution: {:?}", e);
                }
            }
        }
        _ => {
            log::trace!("Client message type with payload: {}", msg_type);
        }
    }
}

async fn intercept_client_message(msg_type: u8, _proxy_type: &str, _metrics: &Arc<Metrics>) {
    {
        log::trace!("Client message type: {}", msg_type);
    }
}

async fn intercept_server_message(msg_type: u8, _proxy_type: &str, metrics: &Arc<Metrics>) {
    match msg_type {
        MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS => {
            log::debug!("Intercepted SubmitSharesSuccess from server");
            // Valid shares are no longer tracked - acceptance rate is computed from submitted - stale
        }
        MESSAGE_TYPE_SUBMIT_SHARES_ERROR => {
            log::debug!("Intercepted SubmitSharesError from server");
            if let Some(counter) = &metrics.stale_shares {
                counter.inc();
                log::info!("Metric recorded: sv2_stale_shares incremented");
            }
        }
        _ => {
            log::trace!("Server message type: {}", msg_type);
        }
    }
}

async fn intercept_server_message_with_payload(
    msg_type: u8,
    mut payload: Vec<u8>,
    _proxy_type: &str,
    metrics: &Arc<Metrics>,
) {
    match msg_type {
        MESSAGE_TYPE_SET_NEW_PREV_HASH => {
            log::debug!("Intercepted SetNewPrevHash from server");
            // Try to parse the message
            match SetNewPrevHash::from_bytes(&mut payload) {
                Ok(msg) => {
                    // Extract the prev hash and convert to hex string
                    let prev_hash_bytes = msg.prev_hash.to_vec();
                    let prev_hash_hex = encode_hex(&prev_hash_bytes);

                    log::info!("SetNewPrevHash: prev_hash={}", prev_hash_hex);

                    // Track prev hash timestamp
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as f64;

                    // Clone metrics for async tasks
                    let last_block_mined_value = metrics.last_block_mined_value.clone();
                    let last_sv2_block_template_value =
                        metrics.last_sv2_block_template_value.clone();
                    let hash_clone = prev_hash_hex.clone();

                    if let Some(gauge_vec) = &metrics.sv2_new_job_prev_hash_timestamp_jdc {
                        // Use fixed label "latest" to keep metric persistent
                        gauge_vec.with_label_values(&["latest"]).set(current_time);
                        log::info!("Metric recorded: sv2_new_job_prev_hash_timestamp_jdc set (prev_hash={}, timestamp={})", prev_hash_hex, current_time);

                        // Schedule metric cleanup after 5 seconds
                        let gauge_vec_clone = gauge_vec.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(5)).await;
                            let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                            log::debug!("Cleaned up sv2_new_job_prev_hash_timestamp_jdc metric");
                        });

                        // Schedule fetching block reward/template value
                        let hash_clone = hash_clone.clone();
                        let last_block_clone = last_block_mined_value.clone();
                        let last_template_clone = last_sv2_block_template_value.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(1)).await;

                            // Fetch the previous template value from Prometheus (2 seconds ago)
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_secs_f64();
                            let timestamp = now - 2.0;
                            let fetch_metric_result = fetch_metric_from_prometheus(
                                "http://10.5.0.50:9090",
                                "sv2_block_template_value",
                                timestamp,
                            )
                            .await;

                            // Fetch the last block reward and set last_block_mined_value
                            if let Ok(reward) = fetch_last_block_reward_with_retries(
                                &reverse_hash(&hash_clone),
                                24,
                                Duration::from_secs(5),
                            )
                            .await
                            {
                                let reward_as_f64 = reward as f64;
                                if let Some(gauge) = &last_block_clone {
                                    gauge.set(reward_as_f64);
                                    log::info!("Metric recorded: last_block_mined_value set (prev_hash={}, reward={})", hash_clone, reward_as_f64);
                                }

                                // Set the fetched metric value for last_sv2_block_template_value
                                if let Ok(value) = fetch_metric_result {
                                    if let Some(gauge) = &last_template_clone {
                                        gauge.set(value);
                                        log::info!("Metric recorded: last_sv2_template_value set (prev_hash={}, value={})", hash_clone, value);
                                    }
                                } else {
                                    log::error!(
                                        "Error fetching previous template value from Prometheus"
                                    );
                                }
                            }
                        });
                    }
                    if let Some(gauge_vec) = &metrics.sv2_new_job_prev_hash_timestamp_pool {
                        // Use fixed label "latest" to keep metric persistent
                        gauge_vec.with_label_values(&["latest"]).set(current_time);
                        log::info!("Metric recorded: sv2_new_job_prev_hash_timestamp_pool set (prev_hash={}, timestamp={})", prev_hash_hex, current_time);

                        // Schedule metric cleanup after 5 seconds
                        let gauge_vec_clone = gauge_vec.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(2)).await;
                            let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                            log::debug!("Cleaned up sv2_new_job_prev_hash_timestamp_pool metric");
                        });

                        // Schedule fetching block reward/template value
                        let hash_clone = hash_clone.clone();
                        let last_block_clone = last_block_mined_value.clone();
                        let last_template_clone = last_sv2_block_template_value.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(1)).await;

                            // Fetch the previous template value from Prometheus (2 seconds ago)
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_secs_f64();
                            let timestamp = now - 2.0;
                            let fetch_metric_result = fetch_metric_from_prometheus(
                                "http://10.5.0.50:9090",
                                "sv2_block_template_value",
                                timestamp,
                            )
                            .await;

                            // Fetch the last block reward and set last_block_mined_value
                            if let Ok(reward) = fetch_last_block_reward_with_retries(
                                &reverse_hash(&hash_clone),
                                24,
                                Duration::from_secs(5),
                            )
                            .await
                            {
                                let reward_as_f64 = reward as f64;
                                if let Some(gauge) = &last_block_clone {
                                    gauge.set(reward_as_f64);
                                    log::info!("Metric recorded: last_block_mined_value set (prev_hash={}, reward={})", hash_clone, reward_as_f64);
                                }

                                // Set the fetched metric value for last_sv2_block_template_value
                                if let Ok(value) = fetch_metric_result {
                                    if let Some(gauge) = &last_template_clone {
                                        gauge.set(value);
                                        log::info!("Metric recorded: last_sv2_template_value set (prev_hash={}, value={})", hash_clone, value);
                                    }
                                } else {
                                    log::error!(
                                        "Error fetching previous template value from Prometheus"
                                    );
                                }
                            }
                        });
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse SetNewPrevHash: {:?}", e);
                }
            }
        }
        MESSAGE_TYPE_NEW_TEMPLATE => {
            log::debug!("Intercepted NewTemplate from server");
            // Try to parse the message to extract coinbase value
            match NewTemplate::from_bytes(&mut payload) {
                Ok(msg) => {
                    let template_id = msg.template_id;
                    let coinbase_value = msg.coinbase_tx_value_remaining;
                    let is_future = msg.future_template;
                    log::info!(
                        "NewTemplate: template_id={}, coinbase_value={}, is_future={}",
                        template_id,
                        coinbase_value,
                        is_future
                    );

                    // Update the block template value metric
                    if let Some(gauge) = &metrics.sv2_block_template_value {
                        gauge.set(coinbase_value as f64);
                        log::info!("Metric recorded: sv2_block_template_value set (template_id={}, value={})", template_id, coinbase_value);
                    }

                    // Update the last template value metric
                    if let Some(gauge) = &metrics.last_sv2_block_template_value {
                        gauge.set(coinbase_value as f64);
                        log::info!("Metric recorded: last_sv2_template_value set (template_id={}, value={})", template_id, coinbase_value);
                    }

                    // Track new template timestamp for "time to get a new job" metric (non-future templates only)
                    // This is recorded by tp-jdc and tp-pool proxies
                    if !is_future {
                        let current_time = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("Time went backwards")
                            .as_millis() as f64;

                        if let Some(gauge_vec) = &metrics.sv2_new_job_timestamp_jdc {
                            // Use a fixed label "latest" instead of template_id to keep metric persistent
                            gauge_vec.with_label_values(&["latest"]).set(current_time);
                            log::info!("Metric recorded: sv2_new_job_timestamp_jdc set (template_id={}, timestamp={} ms)", template_id, current_time);

                            // Schedule metric cleanup after 2 seconds
                            let gauge_vec_clone = gauge_vec.clone();
                            tokio::spawn(async move {
                                sleep(Duration::from_secs(2)).await;
                                let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                                log::debug!("Cleaned up sv2_new_job_timestamp_jdc metric");
                            });
                        }
                        if let Some(gauge_vec) = &metrics.sv2_new_job_timestamp_pool {
                            // Use a fixed label "latest" instead of template_id to keep metric persistent
                            gauge_vec.with_label_values(&["latest"]).set(current_time);
                            log::info!("Metric recorded: sv2_new_job_timestamp_pool set (template_id={}, timestamp={} ms)", template_id, current_time);

                            // Schedule metric cleanup after 2 seconds
                            let gauge_vec_clone = gauge_vec.clone();
                            tokio::spawn(async move {
                                sleep(Duration::from_secs(2)).await;
                                let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                                log::debug!("Cleaned up sv2_new_job_timestamp_pool metric");
                            });
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse NewTemplate: {:?}", e);
                }
            }
        }
        MESSAGE_TYPE_NEW_EXTENDED_MINING_JOB => {
            log::debug!("Intercepted NewExtendedMiningJob from server");
            // Try to parse the message to extract job_id
            match NewExtendedMiningJob::from_bytes(&mut payload) {
                Ok(msg) => {
                    let job_id = msg.job_id;
                    log::info!("NewExtendedMiningJob: job_id={}", job_id);

                    // Track new job timestamp - this is when Translator receives the new job from JDC/Pool
                    let current_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_millis() as f64;

                    if let Some(gauge_vec) = &metrics.sv2_new_job_timestamp_jdc {
                        // Use a fixed label "latest" instead of job_id to keep metric persistent
                        gauge_vec.with_label_values(&["latest"]).set(current_time);
                        log::info!("Metric recorded: sv2_new_job_timestamp_jdc set (job_id={}, timestamp={} ms)", job_id, current_time);

                        // Schedule metric cleanup after 2 seconds
                        let gauge_vec_clone = gauge_vec.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(2)).await;
                            let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                            log::debug!("Cleaned up sv2_new_job_timestamp_jdc metric");
                        });
                    }
                    if let Some(gauge_vec) = &metrics.sv2_new_job_timestamp_pool {
                        // Use a fixed label "latest" to keep metric persistent (same as jdc)
                        gauge_vec.with_label_values(&["latest"]).set(current_time);
                        log::info!("Metric recorded: sv2_new_job_timestamp_pool set (job_id={}, timestamp={} ms)", job_id, current_time);

                        // Schedule metric cleanup after 2 seconds
                        let gauge_vec_clone = gauge_vec.clone();
                        tokio::spawn(async move {
                            sleep(Duration::from_secs(2)).await;
                            let _ = gauge_vec_clone.remove_label_values(&["latest"]);
                            log::debug!("Cleaned up sv2_new_job_timestamp_pool metric");
                        });
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse NewExtendedMiningJob: {:?}", e);
                }
            }
        }
        _ => {
            log::trace!("Server message type: {}", msg_type);
        }
    }
}

struct Metrics {
    // Share metrics
    submitted_shares: Option<Counter>,
    stale_shares: Option<Counter>,
    share_submission_timestamp: Option<GaugeVec>,

    // Template metrics
    sv2_new_job_prev_hash_timestamp_jdc: Option<GaugeVec>,
    sv2_new_job_prev_hash_timestamp_pool: Option<GaugeVec>,
    sv2_new_job_timestamp_jdc: Option<GaugeVec>,
    sv2_new_job_timestamp_pool: Option<GaugeVec>,
    sv2_block_template_value: Option<Gauge>,
    last_block_mined_value: Option<Gauge>,
    last_sv2_block_template_value: Option<Gauge>,

    // Block metrics
    block_propagation_time_through_sv2_jdc: Option<Gauge>,
    block_propagation_time_through_sv2_pool: Option<Gauge>,
    mined_blocks: Option<Counter>,
}

impl Metrics {
    fn new(proxy_type: &str) -> Self {
        let mut metrics = Self {
            submitted_shares: None,
            stale_shares: None,
            share_submission_timestamp: None,
            sv2_new_job_prev_hash_timestamp_jdc: None,
            sv2_new_job_prev_hash_timestamp_pool: None,
            sv2_new_job_timestamp_jdc: None,
            sv2_new_job_timestamp_pool: None,
            sv2_block_template_value: None,
            last_block_mined_value: None,
            last_sv2_block_template_value: None,
            block_propagation_time_through_sv2_jdc: None,
            block_propagation_time_through_sv2_pool: None,
            mined_blocks: None,
        };

        // Initialize metrics based on proxy type
        match proxy_type {
            "pool-jdc" => {
                // Track shares between JDC and Pool
                metrics.submitted_shares = Some(
                    register_counter!(
                        "sv2_submitted_shares",
                        "Total number of SV2 submitted shares"
                    )
                    .unwrap(),
                );
                metrics.stale_shares = Some(
                    register_counter!("sv2_stale_shares", "Total number of SV2 stale shares")
                        .unwrap(),
                );
                metrics.share_submission_timestamp = Some(
                    register_gauge_vec!(
                        "sv2_share_submission_timestamp",
                        "Timestamp of share submission",
                        &["nonce"]
                    )
                    .unwrap(),
                );
            }
            "pool-translator" => {
                // Proxy between Pool and Translator - tracks shares AND NewExtendedMiningJob
                metrics.submitted_shares = Some(
                    register_counter!(
                        "sv2_submitted_shares",
                        "Total number of SV2 submitted shares"
                    )
                    .unwrap(),
                );
                metrics.stale_shares = Some(
                    register_counter!("sv2_stale_shares", "Total number of SV2 stale shares")
                        .unwrap(),
                );
                metrics.share_submission_timestamp = Some(
                    register_gauge_vec!(
                        "sv2_share_submission_timestamp",
                        "Timestamp of share submission",
                        &["nonce"]
                    )
                    .unwrap(),
                );
                metrics.sv2_new_job_timestamp_pool = Some(
                    register_gauge_vec!(
                        "sv2_new_job_timestamp_pool",
                        "Time taken for mining device to get notification of new job via config c",
                        &["id"]
                    )
                    .unwrap(),
                );
            }
            "tp-jdc" => {
                metrics.mined_blocks = Some(
                    register_counter!("sv2_mined_blocks", "Total number of SV2 blocks mined")
                        .unwrap(),
                );
                metrics.block_propagation_time_through_sv2_jdc = Some(
                    register_gauge!(
                        "block_propagation_time_through_sv2_jdc",
                        "Time to submit a block through SV2 JDC in milliseconds"
                    )
                    .unwrap(),
                );
                metrics.sv2_new_job_prev_hash_timestamp_jdc = Some(
                    register_gauge_vec!(
                        "sv2_new_job_prev_hash_timestamp_jdc",
                        "Time taken for mining device to get notification of new prev hash via config a",
                        &["prevhash"]
                    )
                    .unwrap(),
                );
                metrics.sv2_new_job_timestamp_jdc = Some(
                    register_gauge_vec!(
                        "sv2_new_job_timestamp_jdc",
                        "Time taken for mining device to get notification of new job via config a",
                        &["id"]
                    )
                    .unwrap(),
                );
                metrics.sv2_block_template_value = Some(
                    register_gauge!(
                        "sv2_block_template_value",
                        "Total reward of sats contained in the current SV2 block template"
                    )
                    .unwrap(),
                );
                metrics.last_block_mined_value = Some(
                    register_gauge!(
                        "last_block_mined_value",
                        "Total reward of sats contained in the last block mined"
                    )
                    .unwrap(),
                );
                metrics.last_sv2_block_template_value = Some(
                    register_gauge!(
                        "last_sv2_template_value",
                        "Total reward of sats contained in the last SV2 block template"
                    )
                    .unwrap(),
                );
            }
            "tp-pool" => {
                metrics.mined_blocks = Some(
                    register_counter!("sv2_mined_blocks", "Total number of SV2 blocks mined")
                        .unwrap(),
                );
                metrics.block_propagation_time_through_sv2_pool = Some(
                    register_gauge!(
                        "block_propagation_time_through_sv2_pool",
                        "Time to submit a block through SV2 Pool in milliseconds"
                    )
                    .unwrap(),
                );
                metrics.sv2_new_job_prev_hash_timestamp_pool = Some(
                    register_gauge_vec!(
                        "sv2_new_job_prev_hash_timestamp_pool",
                        "Time taken for mining device to get notification of new prev hash via config c",
                        &["prevhash"]
                    )
                    .unwrap(),
                );
                metrics.sv2_new_job_timestamp_pool = Some(
                    register_gauge_vec!(
                        "sv2_new_job_timestamp_pool",
                        "Time taken for mining device to get notification of new job via config c",
                        &["id"]
                    )
                    .unwrap(),
                );
                metrics.sv2_block_template_value = Some(
                    register_gauge!(
                        "sv2_block_template_value",
                        "Total reward of sats contained in the current SV2 block template"
                    )
                    .unwrap(),
                );
                metrics.last_block_mined_value = Some(
                    register_gauge!(
                        "last_block_mined_value",
                        "Total reward of sats contained in the last block mined"
                    )
                    .unwrap(),
                );
                metrics.last_sv2_block_template_value = Some(
                    register_gauge!(
                        "last_sv2_template_value",
                        "Total reward of sats contained in the last SV2 block template"
                    )
                    .unwrap(),
                );
            }
            "jdc-translator" => {
                // Proxy between JDC and Translator - tracks NewExtendedMiningJob only
                // Shares are tracked by pool-jdc sniffer instead
                metrics.sv2_new_job_timestamp_jdc = Some(
                    register_gauge_vec!(
                        "sv2_new_job_timestamp_jdc",
                        "Time taken for mining device to get notification of new job via config a",
                        &["id"]
                    )
                    .unwrap(),
                );
            }
            _ => {
                log::warn!("Unknown proxy type: {}", proxy_type);
            }
        }

        metrics
    }
}

async fn fetch_block_template_value(metrics: Arc<Metrics>) {
    let client = Client::new();
    let prometheus_url = "http://10.5.0.50:9090"; // Updated IP

    loop {
        sleep(Duration::from_secs(60)).await;

        // Fetch the current block template value
        let query = "sv2_block_template_value";
        let url = format!("{}/api/v1/query?query={}", prometheus_url, query);

        match client.get(&url).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(result) = json["data"]["result"].as_array() {
                        if let Some(first) = result.first() {
                            if let Some(value_str) = first["value"][1].as_str() {
                                if let Ok(value) = value_str.parse::<f64>() {
                                    if let Some(gauge) = &metrics.last_sv2_block_template_value {
                                        gauge.set(value);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::debug!("Failed to fetch block template value: {}", e);
            }
        }
    }
}
