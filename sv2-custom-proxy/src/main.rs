use demand_easy_sv2::const_sv2::{MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS, MESSAGE_TYPE_SUBMIT_SHARES_ERROR};
use demand_easy_sv2::roles_logic_sv2::parsers::{Mining, PoolMessages};
use demand_easy_sv2::{ProxyBuilder, Remote};
use prometheus::{register_counter, Counter, Encoder, TextEncoder};
use std::env;
use std::net::ToSocketAddrs;
use tokio::net::TcpStream;
use warp::Filter;

#[tokio::main]
async fn main() {
    // Load environment variables
    let client_address = env::var("CLIENT").expect("CLIENT environment variable not set");
    let server_address = env::var("SERVER").expect("SERVER environment variable not set");

    // Create Prometheus counters
    let submitted_shares = register_counter!(
        "sv2_submitted_shares",
        "Total number of SV2 submitted shares"
    ).unwrap();

    let valid_shares = register_counter!(
        "sv2_valid_shares",
        "Total number of SV2 valid shares"
    ).unwrap();

    let stale_shares = register_counter!(
        "sv2_stale_shares",
        "Total number of SV2 stale shares"
    ).unwrap();

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

        warp::serve(metrics_route).run(([0, 0, 0, 0], 3456)).await;
    });

    let mut proxy_builder = ProxyBuilder::new();
    proxy_builder
        .try_add_client(listen_for_client(&client_address).await)
        .await
        .unwrap()
        .try_add_server(connect_to_server(&server_address).await)
        .await
        .unwrap();

    intercept_submit_share_extended(&mut proxy_builder, submitted_shares.clone()).await;
    intercept_submit_share_success(&mut proxy_builder, valid_shares.clone()).await;
    intercept_submit_share_error(&mut proxy_builder, stale_shares.clone()).await;

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

async fn intercept_submit_share_extended(builder: &mut ProxyBuilder, submitted_shares: Counter) {
    let mut r =
        builder.add_handler(Remote::Client, MESSAGE_TYPE_SUBMIT_SHARES_EXTENDED);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesExtended(m))) = r.recv().await {
            println!("SubmitSharesExtended received --> {:?}", m);
            submitted_shares.inc();
        }
    });
}

async fn intercept_submit_share_success(builder: &mut ProxyBuilder, valid_shares: Counter) {
    let mut r =
        builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_SUCCESS);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesSuccess(m))) = r.recv().await {
            println!("SubmitSharesSuccess received --> {:?}", m);
            valid_shares.inc();
        }
    });
}

async fn intercept_submit_share_error(builder: &mut ProxyBuilder, stale_shares: Counter) {
    let mut r =
        builder.add_handler(Remote::Server, MESSAGE_TYPE_SUBMIT_SHARES_ERROR);
    tokio::spawn(async move {
        while let Some(PoolMessages::Mining(Mining::SubmitSharesError(m))) = r.recv().await {
            println!("SubmitSharesError received --> {:?}", m);
            stale_shares.inc();
        }
    });
}
