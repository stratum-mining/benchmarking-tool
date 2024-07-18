use hyper::{Body, Client, Request, Response, Server, Uri};
use hyper::service::{make_service_fn, service_fn};
use tokio::time::{sleep, Duration};
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use serde_json::Value;
use prometheus::{register_counter, register_gauge, register_gauge_vec, Counter, Encoder, Gauge, GaugeVec, TextEncoder};
use warp::Filter;

async fn transfer(mut inbound: TcpStream, mut outbound: TcpStream, submitted_shares: Counter, valid_shares: Counter, stale_shares: Counter, share_submission_timestamp: GaugeVec) -> io::Result<()> {
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = async {
        let mut buf = vec![0; 4096];
        let mut client_buf = Vec::new();
        loop {
            let n = ri.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            client_buf.extend_from_slice(&buf[..n]);
            while let Some(pos) = client_buf.iter().position(|&b| b == b'\n') {
                let line = client_buf.drain(..=pos).collect::<Vec<_>>();
                if let Ok(json) = serde_json::from_slice::<Value>(&line) {
                    println!("Client to Server: {}", json);
                    if json["method"] == "mining.submit" {
                        submitted_shares.inc();
                        if let Some(params) = json["params"].as_array() {
                            if let Some(nonce) = params.get(4) {
                                let nonce_string = nonce.to_string();  // Clone the nonce to a String
                                let current_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                    .expect("Time went backwards")
                                    .as_millis() as f64;
                                // Clone the gauge for use in the task
                                let share_submission_timestamp_clone = share_submission_timestamp.clone();
                                share_submission_timestamp_clone.with_label_values(&[&nonce_string]).set(current_time);

                                // Spawn a task to remove the gauge after 10 seconds
                                tokio::spawn(async move {
                                    sleep(Duration::from_secs(10)).await;
                                    // Remove the metric from Prometheus
                                    let _ = share_submission_timestamp_clone.remove_label_values(&[&nonce_string]);
                                });

                            } else {
                                println!("Nonce not found in params");
                            }
                        } else {
                            println!("Params is not an array");
                        }
                    }
                } else {
                    println!("Client to Server: {:?}", line);
                }
                wo.write_all(&line).await?;
            }
        }
        wo.shutdown().await
    };

    let server_to_client = async {
        let mut buf = vec![0; 4096];
        let mut server_buf = Vec::new();
        let mut first_result_seen = false;
        loop {
            let n = ro.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            server_buf.extend_from_slice(&buf[..n]);
            while let Some(pos) = server_buf.iter().position(|&b| b == b'\n') {
                let line = server_buf.drain(..=pos).collect::<Vec<_>>();
                if let Ok(json) = serde_json::from_slice::<Value>(&line) {
                    println!("Server to Client: {}", json);
                    if !first_result_seen && json["result"] == true {
                        first_result_seen = true;
                        println!("Not counting this as a valid share because it's related to mining.subscribe");
                    } else if json["result"] == true {
                        valid_shares.inc();
                    } else if json["error"].is_array() {
                        stale_shares.inc();
                    }
                } else {
                    println!("Server to Client: {:?}", line);
                }
                wi.write_all(&line).await?;
            }
        }
        wi.shutdown().await
    };

    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}

async fn handle_rpc_request(req: Request<Body>, forward_uri: Uri, latency_metric: Gauge) -> Result<Response<Body>, hyper::Error> {
    // Log the incoming request
    let uri = req.uri().clone();
    let method = req.method().clone();
    let headers = req.headers().clone();
    let body_bytes = hyper::body::to_bytes(req.into_body()).await?;
    let body_str = String::from_utf8_lossy(&body_bytes);
    println!("Incoming request: {} {} {:?}", method, uri, headers);
    println!("Request body: {}", body_str);
    
    // Parse the request body as JSON
    if let Ok(json) = serde_json::from_slice::<Value>(&body_bytes) {
        if let Some(method) = json.get("method") {
            if method == "submitblock" {
                println!("Detected submitblock method.");
                // Perform Prometheus query
                let prometheus_url = "http://10.5.0.19:2345/metrics";
                let client = reqwest::Client::new();
                if let Ok(response) = client.get(prometheus_url).send().await {
                    if let Ok(body) = response.text().await {
                        // Flag to track if nonce is found
                        let mut nonce_found = false;
                        
                        // Iterate over each line to find share_submission_timestamp metrics
                        for line in body.lines() {
                            if let Some(start_index) = line.find("nonce=\"\\\"") {
                                let start = start_index + "nonce=\"\\\"".len();
                                let end = match line[start..].find("\\\"") {
                                    Some(index) => start + index,
                                    None => {
                                        println!("Failed to find end quote for nonce in line: {}", line);
                                        continue; // Skip to the next line if end quote for nonce is not found
                                    }
                                };
                                let nonce_value = &line[start..end];
                                // Decode the nonce hex string into bytes
                                let nonce_bytes_result = hex::decode(&nonce_value);
                                // Check if the conversion succeeded
                                let nonce_bytes = match nonce_bytes_result {
                                    Ok(bytes) => bytes,
                                    Err(e) => {
                                        println!("Failed to parse nonce hex: {}", e);
                                        continue; // Skip this line if nonce parsing fails
                                    }
                                };
                                // Perform further actions with the nonce bytes
                                let swapped_bytes: Vec<u8> = nonce_bytes.iter().rev().cloned().collect();
                                let swapped_nonce = hex::encode_upper(&swapped_bytes).to_lowercase();
                        
                                // Check if the body contains the swapped nonce
                                if body_str.contains(&swapped_nonce) {
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
                                    // Set the flag indicating nonce found
                                    nonce_found = true;
                                    break; // Exit the loop since nonce is found
                                }
                            }
                        }
                        // Check if nonce was not found in any line
                        if !nonce_found {
                            println!("Nonce not found in Prometheus metrics");
                        }
                    }
                }
            }
        }
    }

    // Create a new client
    let client = Client::new();

    // Forward the request to the actual server
    let mut new_req = Request::builder()
        .method(method)
        .uri(forward_uri)
        .body(Body::from(body_bytes.clone()))  // Clone the body bytes for forwarding
        .expect("request builder");

    *new_req.headers_mut() = headers.clone();

    // Add authentication header if not already present
    if !new_req.headers().contains_key("authorization") {
        let auth_value = "Basic dXNlcm5hbWU6cGFzc3dvcmQ=";
        new_req.headers_mut().insert("authorization", auth_value.parse().unwrap());
    }

    // Log the forwarded request
    let forwarded_headers = new_req.headers().clone();
    println!("Forwarded request: {} {} {:?}", new_req.method(), new_req.uri(), forwarded_headers);
    println!("Forwarded request body: {}", body_str);

    let res = match client.request(new_req).await {
        Ok(res) => res,
        Err(err) => {
            println!("Error forwarding request: {}", err);
            return Err(err);
        }
    };

    // Log the response
    let status = res.status();
    let headers = res.headers().clone();
    let body_bytes = hyper::body::to_bytes(res.into_body()).await?;
    let body_str = String::from_utf8_lossy(&body_bytes);
    println!("Response: {} {:?}", status, headers);
    println!("Response body: {}", body_str);

    // Create a new response to return to the client
    let new_res = Response::builder()
        .status(status)
        .body(Body::from(body_bytes))
        .expect("response builder");

    Ok(new_res)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read environment variables
    let proxy_type = env::var("PROXY_TYPE").expect("PROXY_TYPE environment variable not set");
    let client = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server = env::var("SERVER").expect("SERVER environment variable not set");
    let prometheus_exporter_address = env::var("PROM_ADDRESS").expect("PROM_ADDRESS environment variable not set");

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

    if proxy_type == "pool-miner" {
        let submitted_shares = register_counter!(
            "sv1_submitted_shares",
            "Total number of SV1 submitted shares"
        )?;

        let valid_shares = register_counter!(
            "sv1_valid_shares",
            "Total number of SV1 valid shares"
        )?;

        let stale_shares = register_counter!(
            "sv1_stale_shares",
            "Total number of SV1 stale shares"
        )?;
        let share_submission_timestamp = register_gauge_vec!(
            "share_submission_timestamp",
            "Timestamp of the submitted share",
            &["nonce"]
        )?;

        let client_address: SocketAddr = client.parse().expect("Invalid address");
        let server_address: SocketAddr = server.parse().expect("Invalid address");
        let listener = TcpListener::bind(client_address).await?;
        println!("SV1 proxy listening on port 3333");

        loop {
            let (inbound, _) = listener.accept().await?;
            let outbound = TcpStream::connect(server_address).await?;
            
            let submitted_shares_clone = submitted_shares.clone();
            let valid_shares_clone = valid_shares.clone();
            let stale_shares_clone = stale_shares.clone();
            let share_submission_timestamp_clone = share_submission_timestamp.clone();
            
            tokio::spawn(async move {
                if let Err(e) = transfer(inbound, outbound, submitted_shares_clone, valid_shares_clone, stale_shares_clone, share_submission_timestamp_clone).await {
                    println!("Failed to transfer; error = {}", e);
                }
            });
        }
    } else { // this means proxy is between sv1-node and sv1-pool
        let addr = env::var("CLIENT").expect("CLIENT environment variable not set");
        let forward_uri = env::var("SERVER").expect("SERVER_URI environment variable not set");
        let addr: SocketAddr = addr.parse().expect("Invalid address");
        let forward_uri: Uri = forward_uri.parse().expect("Invalid URI");

        let block_propagation_time_through_sv1_pool_latency = register_gauge!(
            "block_propagation_time_through_sv1_pool_latency",
            "Latency for submit solution through SV1 Pool in milliseconds"
        )?;

        let make_svc = make_service_fn(move |_conn| {
            let forward_uri = forward_uri.clone();
            let latency_metric = block_propagation_time_through_sv1_pool_latency.clone(); // Clone the gauge
            async move {
                Ok::<_, Infallible>(service_fn(move |req| handle_rpc_request(req, forward_uri.clone(), latency_metric.clone())))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        println!("Listening on http://{}", addr);

        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    }

    Ok(())
}
