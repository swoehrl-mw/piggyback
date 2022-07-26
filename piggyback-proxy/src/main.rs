use piggyback_common::proxy_sockets;
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "8080".to_string());

    // Start listener on control port (12345)
    let control_listener = TcpListener::bind("[::]:12345").await.unwrap();
    println!("Waiting for client connections");

    loop {
        handle_connection(&port, &control_listener).await;
    }
}

async fn handle_connection(port: &String, control_listener: &TcpListener) {
    // Wait for connection
    let (mut control_socket, _) = control_listener.accept().await.unwrap();
    println!("Client connected");

    // When connection is established, start listening on proxy port
    let proxy_listener = TcpListener::bind(format!("[::]:{}", port)).await.unwrap();
    println!("Started proxy port");
    loop {
        let (mut proxy_socket, _) = proxy_listener.accept().await.unwrap();
        println!("Got connection. Proxying data");
        // Do two-way copy of all data
        proxy_sockets(&mut control_socket, &mut proxy_socket)
            .await
            .unwrap();
        println!("Proxy done");
    }
}
