use std::{env, net::ToSocketAddrs};

use demand_easy_sv2::{const_sv2::MESSAGE_TYPE_SET_NEW_PREV_HASH, roles_logic_sv2::parsers::TemplateDistribution, PoolMessages, ProxyBuilder};
use prometheus::{register_counter, Counter, Encoder, TextEncoder};
use tokio::net::TcpStream;
use warp::Filter;



#[tokio::main]
async fn main() {
    // load environment variables
    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");

    // Creating Prometheus counters
    let count_new_prevhash = register_counter!("sv2_prev_hash","Total number of prev_hash from TP").unwrap();

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
    let mut proxy_builder = ProxyBuilder::new();
    proxy_builder
        .try_add_client(listen_for_client(&client_address).await)
        .await
        .unwrap()
        .try_add_server(connect_to_server(&server_address).await)
        .await
        .unwrap();
    intercept_prev_hash(&mut proxy_builder, count_new_prevhash.clone()).await;

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

async fn intercept_prev_hash(builder: &mut ProxyBuilder, count_prev_hash: Counter) {
    let mut r = builder.add_handler(demand_easy_sv2::Remote::Server, MESSAGE_TYPE_SET_NEW_PREV_HASH);
    tokio::spawn(async move {
        while let Some(PoolMessages::TemplateDistribution(TemplateDistribution::SetNewPrevHash(m))) = r.recv().await{
            println!("Set prev hash received --> {:?}",m);
            count_prev_hash.inc();            
        }
    });
}