use std::{
    collections::VecDeque,
    env,
    net::ToSocketAddrs,
    sync::{Arc, Mutex},
    time::Instant,
};

use demand_easy_sv2::{
    const_sv2::MESSAGE_TYPE_SET_NEW_PREV_HASH, roles_logic_sv2::parsers::TemplateDistribution,
    PoolMessages, ProxyBuilder,
};
use prometheus::{
    core::{AtomicF64, GenericGauge},
    register_gauge, Encoder, TextEncoder,
};
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime::Builder,
};
use warp::Filter;

#[tokio::main]
async fn main() {
    // load environment variables
    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");

    let gauge = Arc::new(Mutex::new(
        register_gauge!(
            "sv2_new_job_latency_milliseconds",
            "New Job sv2 latency in milliseconds"
        )
        .unwrap(),
    ));

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

        warp::serve(metrics_route).run(([0, 0, 0, 0], 3477)).await;
    });

    let separate_runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();

    let timer_deque: Arc<Mutex<VecDeque<Instant>>> = Arc::new(Mutex::new(VecDeque::new()));
    let timer_deque_2 = timer_deque.clone();
    std::thread::spawn(move || {
        separate_runtime.block_on(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:34255")
                .await
                .unwrap();
            println!("SV2 proxy translation proxy started at 34255");
            loop {
                let (inbound, _) = listener.accept().await.unwrap();
                let outbound = TcpStream::connect("10.5.0.7:34256").await.unwrap();
                let timer_deque_clone1 = timer_deque_2.clone();
                let gauge_metric = gauge.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        transfer(inbound, outbound, timer_deque_clone1, gauge_metric).await
                    {
                        println!("Failed to transfer; error = {}", e);
                    }
                });
            }
        });
    });

    let mut proxy_builder = ProxyBuilder::new();
    proxy_builder
        .try_add_client(listen_for_client(&client_address).await)
        .await
        .unwrap()
        .try_add_server(connect_to_server(&server_address).await)
        .await
        .unwrap();
    intercept_prev_hash(&mut proxy_builder, timer_deque.clone()).await;

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
    let res = TcpStream::connect(address).await.unwrap();
    res
}

async fn intercept_prev_hash(
    builder: &mut ProxyBuilder,
    timer_deque: Arc<Mutex<VecDeque<Instant>>>,
) {
    let mut r = builder.add_handler(
        demand_easy_sv2::Remote::Server,
        MESSAGE_TYPE_SET_NEW_PREV_HASH,
    );
    let time_deque_clone = timer_deque.clone();
    tokio::spawn(async move {
        while let Some(PoolMessages::TemplateDistribution(TemplateDistribution::SetNewPrevHash(
            m,
        ))) = r.recv().await
        {
            println!("Set prev hash received --> {:?}", m);
            let t = Instant::now();
            println!("Timer added");
            time_deque_clone.lock().unwrap().push_back(t);
        }
    });
}

async fn transfer(
    mut inbound: tokio::net::TcpStream,
    mut outbound: tokio::net::TcpStream,
    timer_deque: Arc<Mutex<VecDeque<Instant>>>,
    gauge: Arc<Mutex<GenericGauge<AtomicF64>>>,
) -> std::io::Result<()> {
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
                wo.write_all(&line).await?;
            }
        }
        wo.shutdown().await
    };

    let server_to_client = async {
        let mut buf = vec![0; 4096];
        let mut server_buf = Vec::new();
        loop {
            let timer_deque_clone = timer_deque.clone();
            let n = ro.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            server_buf.extend_from_slice(&buf[..n]);
            while let Some(pos) = server_buf.iter().position(|&b| b == b'\n') {
                let line = server_buf.drain(..=pos).collect::<Vec<_>>();
                if let Ok(json) = serde_json::from_slice::<Value>(&line) {
                    if json["method"] == "mining.notify" {
                        let value = timer_deque_clone.clone().lock().unwrap().pop_front();
                        if let Some(timer) = value {
                            println!("Timer removed");
                            gauge
                                .lock()
                                .unwrap()
                                .set(timer.elapsed().as_secs_f64() * 1000.0);
                        }
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
