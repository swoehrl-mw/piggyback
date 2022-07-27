use piggyback_common::{proxy_sockets, ClosedSocket};
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use std::{thread, time::Duration};
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Make sure we exit on Ctrl-C
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
    thread::spawn(move || {
        for _ in signals.forever() {
            std::process::exit(0);
        }
    });

    let port = std::env::var("PROXY_PORT").unwrap_or_else(|_| "8080".to_string());

    // Start listener on control port (12345)
    let control_listener = TcpListener::bind("[::]:12345").await.unwrap();

    loop {
        println!("Waiting for client connections");
        handle_connection(&port, &control_listener).await;
    }
}

async fn handle_connection(port: &String, control_listener: &TcpListener) {
    // Wait for connection
    let mut buffer = vec![0; 1];
    let (mut control_socket, _) = control_listener.accept().await.unwrap();
    println!("Client connected");

    // When connection is established, start listening on proxy port
    let proxy_listener = TcpListener::bind(format!("[::]:{}", port)).await.unwrap();
    println!("Started proxy port");
    loop {
        // While waiting for a new connection on the proxy port, periodically check if the control socket is still active (size > 0 or Err(io::ErrorKind::WouldBlock))
        let (mut proxy_socket, _) = match tokio::time::timeout(Duration::from_secs(5), proxy_listener.accept()).await {
            Ok(res) => res.unwrap(),
            Err(_) => {
                if let Ok(size) = control_socket.try_read(&mut buffer) {
                    if size == 0 {
                        println!("Client disconnected");
                        return;
                    }
                }
                continue;
            }
        };
        println!("Got connection. Proxying data");
        // Do two-way copy of all data
        match proxy_sockets(&mut control_socket, &mut proxy_socket).await {
            Ok(closed_socket) => {
                println!("Proxy done");
                if closed_socket == ClosedSocket::First {
                    println!("Client disconnected");
                    return;
                }
            }
            Err(err) => {
                println!("Got error: {}", err);
                return;
            }
        };
    }
}
