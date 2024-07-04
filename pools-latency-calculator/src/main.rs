use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use serde_json::json;
use prometheus::{Encoder, TextEncoder, register_gauge, Gauge};
use std::net::SocketAddr;
use warp::Filter;

const TIMEOUT_DURATION: Duration = Duration::from_secs(10);
const SLEEP_DURATION: Duration = Duration::from_secs(60);

async fn connect_to_pool(url: &str) -> Result<TcpStream, std::io::Error> {
    let url_parts: Vec<&str> = url.split(':').collect();
    let host = &url_parts[1][2..];
    let port: u16 = url_parts[2].parse().unwrap();

    TcpStream::connect((host, port)).await
}

async fn subscribe_to_pool(mut stream: TcpStream) -> Result<Duration, std::io::Error> {
    let subscribe_msg = json!({
        "id": 1,
        "method": "mining.subscribe",
        "params": []
    }).to_string() + "\n";

    let start = Instant::now();
    stream.write_all(subscribe_msg.as_bytes()).await?;
    let mut buffer = vec![0; 2028];
    stream.read(&mut buffer).await?;
    Ok(start.elapsed())
}

async fn get_subscription_latency(address: &str) -> Result<Duration, std::io::Error> {
    match connect_to_pool(address).await {
        Ok(connection) => {
            match timeout(TIMEOUT_DURATION, subscribe_to_pool(connection)).await {
                Ok(result) => result,
                Err(_) => {
                    println!("Timeout while subscribing to {}", address);
                    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Subscription timeout"))
                }
            }
        }
        Err(e) => {
            println!("Failed to connect to {}: {}", address, e);
            Err(e)
        }
    }
}

async fn average_latency(addresses: Vec<&str>, repetitions: usize, gauge: Gauge) {
    let mut total_duration = Duration::new(0, 0);
    let mut total_attempts = 0;

    for address in addresses {
        println!("Starting latency measurement for: {}", address);
        let mut address_duration = Duration::new(0, 0);

        for i in 0..repetitions {
            print!("Attempt {} for {}... ", i + 1, address);
            match get_subscription_latency(address).await {
                Ok(duration) => {
                    address_duration += duration;
                    println!("latency: {:?}", duration);
                }
                Err(e) => println!("Error: {}", e),
            }
            sleep(Duration::from_millis(100)).await;
            total_attempts += 1;
        }

        let avg_address_duration = address_duration / repetitions as u32;
        total_duration += address_duration;
        println!("Average latency for {}: {:?}", address, avg_address_duration);
    }

    let avg_total_duration = total_duration / total_attempts as u32;
    println!("Total average latency: {:?}", avg_total_duration);

    let avg_total_duration_ms = avg_total_duration.as_secs_f64() * 1000.0;
    gauge.set(avg_total_duration_ms);
}

#[tokio::main]
async fn main() {
    let addresses = vec![
        "stratum+tcp://ss.antpool.com:3333",
        "stratum+tcp://ss.antpool.com:443",
        "stratum+tcp://btc.viabtc.io:3333",
        "stratum+tcp://btc.viabtc.io:443",
        "stratum+tcp://btc.f2pool.com:3333",
        "stratum+tcp://eu1.sbicrypto.com:3333",
        "stratum+tcp://bs.poolbinance.com:3333",
        "stratum+tcp://sha256.poolbinance.com:8888",
        "stratum+tcp://stratum.braiins.com:3333",
        "stratum+tcp://us.ss.btc.com:1800",
        "stratum+tcp://eu.ss.btc.com:1800",
        "stratum+tcp://sg.ss.btc.com:1800",
        "stratum+tcp://btc.global.luxor.tech:700",
        "stratum+tcp://btc.secpool.com:3333",
        "stratum+tcp://btc.secpool.com:443",
        "stratum+tcp://mine.ocean.xyz:3334"
    ];

    let repetitions = 10;

    let gauge = register_gauge!(
        "average_pool_subscription_latency_milliseconds",
        "Average subscription latency to various mining pools in milliseconds"
    ).unwrap();

    // Start the latency measurement in a loop
    tokio::spawn(async move {
        loop {
            average_latency(addresses.clone(), repetitions, gauge.clone()).await;
            sleep(SLEEP_DURATION).await;
        }
    });

    // Start the Prometheus metrics server
    let metrics_route = warp::path("metrics").map(move || {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        warp::http::Response::builder()
            .header("Content-Type", encoder.format_type())
            .body(buffer)
    });

    let addr: SocketAddr = ([0, 0, 0, 0], 1234).into();
    println!("Starting Prometheus metrics server on http://0.0.0.0:1234/metrics");
    warp::serve(metrics_route).run(addr).await;
}
