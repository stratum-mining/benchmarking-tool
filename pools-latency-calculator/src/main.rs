use prometheus::{register_gauge, Encoder, Gauge, TextEncoder};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use warp::Filter;

const TIMEOUT_DURATION: Duration = Duration::from_secs(10);
const SLEEP_DURATION: Duration = Duration::from_secs(60);

async fn connect_to_pool(url: &str) -> Result<TcpStream, std::io::Error> {
    let url_parts: Vec<&str> = url.split(':').collect();
    let host = &url_parts[1][2..];
    let port: u16 = url_parts[2].parse().unwrap();

    TcpStream::connect((host, port)).await.map_err(|e| {
        log::error!("Failed to connect to pool {}: {}", url, e);
        e
    })
}

async fn subscribe_to_pool(mut stream: TcpStream) -> Result<Duration, std::io::Error> {
    let subscribe_msg = json!({
        "id": 1,
        "method": "mining.subscribe",
        "params": []
    })
    .to_string()
        + "\n";

    let start = Instant::now();
    stream.write_all(subscribe_msg.as_bytes()).await?;

    let reader = tokio::io::BufReader::new(&mut stream);
    match reader.lines().next_line().await {
        Ok(Some(_)) => Ok(start.elapsed()),
        Ok(None) => {
            log::error!("Received empty response");
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Empty response",
            ))
        }
        Err(e) => {
            log::error!("Failed to receive subscription response: {}", e);
            Err(e)
        }
    }
}

async fn get_subscription_latency(address: &str) -> Result<Duration, std::io::Error> {
    log::info!("Measuring subscription latency for: {}", address);
    match connect_to_pool(address).await {
        Ok(connection) => match timeout(TIMEOUT_DURATION, subscribe_to_pool(connection)).await {
            Ok(result) => result,
            Err(_) => {
                log::error!("Timeout while subscribing to {}", address);
                Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Subscription timeout",
                ))
            }
        },
        Err(e) => {
            log::error!("Failed to connect to {}: {}", address, e);
            Err(e)
        }
    }
}

async fn average_latency(pool_map: HashMap<&str, Vec<&str>>, repetitions: usize, gauge: Gauge) {
    let mut total_duration = Duration::new(0, 0);
    let mut total_pools = 0;

    for (pool_name, addresses) in pool_map.iter() {
        log::info!("Starting latency measurement for pool: {}", pool_name);
        let mut pool_duration = Duration::new(0, 0);

        for address in addresses {
            let mut address_duration = Duration::new(0, 0);
            for i in 0..repetitions {
                log::info!("Attempt {} for {}...", i + 1, address);
                match get_subscription_latency(address).await {
                    Ok(duration) => {
                        address_duration += duration;
                    }
                    Err(e) => log::error!("Error in attempt {}: {}", i + 1, e),
                }
                sleep(Duration::from_millis(100)).await;
            }
            pool_duration += address_duration / repetitions as u32;
        }

        let avg_pool_duration = pool_duration / addresses.len() as u32;
        total_duration += avg_pool_duration;
        total_pools += 1;
        log::info!(
            "Average latency for pool {}: {:?}\n",
            pool_name,
            avg_pool_duration
        );
    }

    let avg_total_duration = total_duration / total_pools as u32;
    log::info!(
        "Total average latency across pools: {:?}",
        avg_total_duration
    );

    let avg_total_duration_ms = avg_total_duration.as_secs_f64() * 1000.0;
    gauge.set(avg_total_duration_ms);
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

    let pool_map: HashMap<&str, Vec<&str>> = HashMap::from([
        (
            "F2Pool",
            vec![
                "stratum+tcp://btc.f2pool.com:1314",
                "stratum+tcp://btc-asia.f2pool.com:1314",
                "stratum+tcp://btc-na.f2pool.com:1314",
                "stratum+tcp://btc-euro.f2pool.com:1314",
                "stratum+tcp://btc-africa.f2pool.com:1314",
                "stratum+tcp://btc-latin.f2pool.com:1314",
            ],
        ),
        ("Secpool", vec!["stratum+tcp://btc.secpool.com:3333"]),
        (
            "Spiderpool",
            vec![
                "stratum+tcp://btc-eu.spiderpool.com:2309",
                "stratum+tcp://btc-us.spiderpool.com:2309",
                "stratum+tcp://btc-as.spiderpool.com:2309",
            ],
        ),
        ("Luxor", vec!["stratum+tcp://btc.global.luxor.tech:700"]),
        (
            "Binance",
            vec![
                "stratum+tcp://bs.poolbinance.com:3333",
                "stratum+tcp://sha256.poolbinance.com:8888",
            ],
        ),
        ("Braiins", vec!["stratum+tcp://stratum.braiins.com:3333"]),
        ("Ocean", vec!["stratum+tcp://mine.ocean.xyz:3334"]),
        ("Antpool", vec!["stratum+tcp://ss.antpool.com:3333"]),
        ("Viabtc", vec!["stratum+tcp://btc.viabtc.io:3333"]),
    ]);

    let repetitions = 10;

    let gauge = register_gauge!(
        "average_pool_subscription_latency_milliseconds",
        "Average subscription latency to various mining pools in milliseconds"
    )
    .unwrap();

    tokio::spawn(async move {
        loop {
            average_latency(pool_map.clone(), repetitions, gauge.clone()).await;
            sleep(SLEEP_DURATION).await;
        }
    });

    let addr: SocketAddr = ([0, 0, 0, 0], 1234).into();
    log::info!("Starting Prometheus metrics server on http://0.0.0.0:1234/metrics");
    let metrics_route = warp::path("metrics").map(move || {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        warp::http::Response::builder()
            .header("Content-Type", encoder.format_type())
            .body(buffer)
    });

    log::info!("Starting Prometheus metrics server on http://0.0.0.0:1234/metrics");
    warp::serve(metrics_route).run(addr).await;
}
