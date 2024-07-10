use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use serde_json::Value;
use prometheus::{register_counter, Counter, Encoder, TextEncoder};
use warp::Filter;

async fn transfer(mut inbound: TcpStream, mut outbound: TcpStream, submitted_shares: Counter, valid_shares: Counter, stale_shares: Counter) -> io::Result<()> {
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


#[tokio::main]
async fn main() -> io::Result<()> {
    let submitted_shares = register_counter!(
        "sv1_submitted_shares",
        "Total number of SV1 submitted shares"
    ).unwrap();

    let valid_shares = register_counter!(
        "sv1_valid_shares",
        "Total number of SV1 valid shares"
    ).unwrap();

    let stale_shares = register_counter!(
        "sv1_stale_shares",
        "Total number of SV1 stale shares"
    ).unwrap();

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

        warp::serve(metrics_route).run(([0, 0, 0, 0], 2345)).await;
    });

    let listener = TcpListener::bind("0.0.0.0:3333").await?;
    println!("SV1 proxy listening on port 3333");

    loop {
        let (inbound, _) = listener.accept().await?;
        let outbound = TcpStream::connect("10.5.0.8:3332").await?;
        
        let submitted_shares_clone = submitted_shares.clone();
        let valid_shares_clone = valid_shares.clone();
        let stale_shares_clone = stale_shares.clone();
        
        tokio::spawn(async move {
            if let Err(e) = transfer(inbound, outbound, submitted_shares_clone, valid_shares_clone, stale_shares_clone).await {
                println!("Failed to transfer; error = {}", e);
            }
        });
    }
}
