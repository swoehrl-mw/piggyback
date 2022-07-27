mod kubernetes;

use argh::FromArgs;
use piggyback_common::proxy_sockets;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::thread;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(FromArgs, PartialEq, Debug)]
/// piggyback
struct MainArgs {
    #[argh(subcommand)]
    command: CommandEnum,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum CommandEnum {
    PortForward(PortForwardArgs),
    Deploy(DeployArgs),
    Delete(DeleteArgs),
    Version(VersionArgs),
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "port-forward")]
/// port-forward
pub struct PortForwardArgs {
    /// target to reverse-proxy, e.g. localhost:8080
    #[argh(positional)]
    target: String,
    /// proxy to connect to, defaults to 'localhost:12345'
    #[argh(option)]
    proxy: Option<String>,
    /// deploys the proxy
    #[argh(switch, short = 'd')]
    deploy: bool,
    /// the name to use for the proxy, defaults to 'piggyback'
    #[argh(option)]
    name: Option<String>,
    /// the namespace to use for the proxy, defaults to the namespace of the kubectl context
    #[argh(option)]
    namespace: Option<String>,
    /// port to use for the proxy, defaults to 8080
    #[argh(option)]
    port: Option<u32>,
    /// override the docker image to be used for the proxy pod
    #[argh(option)]
    proxy_image: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "deploy")]
/// deploy the piggyback proxy
pub struct DeployArgs {
    /// the name to use for the proxy, defaults to 'piggyback'
    #[argh(option)]
    name: Option<String>,
    /// the namespace to use for the proxy, defaults to the namespace of the kubectl context
    #[argh(option)]
    namespace: Option<String>,
    /// port to use for the proxy, defaults to 8080
    #[argh(option)]
    port: Option<u32>,
    /// override the docker image to be used for the proxy pod
    #[argh(option)]
    proxy_image: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "delete")]
/// delete a deployed piggyback proxy
pub struct DeleteArgs {
    /// the name to use for the proxy, defaults to 'piggyback'
    #[argh(option)]
    name: Option<String>,
    /// the namespace to use for the proxy, defaults to the namespace of the kubectl context
    #[argh(option)]
    namespace: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "version")]
/// show the version of piggyback
pub struct VersionArgs {}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: MainArgs = argh::from_env();

    // Make sure we exit on Ctrl-C
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let mut signals = Signals::new(&[SIGINT, SIGTERM]).unwrap();
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    thread::spawn(move || {
        for _ in signals.forever() {
            std::process::exit(0);
        }
    });

    match args.command {
        CommandEnum::PortForward(args) => portforward(args).await,
        CommandEnum::Deploy(args) => deploy(args).await,
        CommandEnum::Delete(args) => delete(args).await,
        CommandEnum::Version(args) => version(args),
    }
}

async fn portforward(args: PortForwardArgs) {
    let name = args.name.unwrap_or_else(|| "piggyback".to_string());
    if args.deploy {
        let port = args.port.unwrap_or(8080);
        kubernetes::deploy_proxy(&name, args.namespace.clone(), port, args.proxy_image).await;
    }

    // kubect-port-forward
    let mut proxy_stream = kubernetes::portforward(&name, args.namespace).await;

    // Connect to proxy
    // let proxy_addr = args.proxy.unwrap_or_else(|| "localhost:12345".to_string());
    // let mut proxy_socket = TcpStream::connect(&proxy_addr).await.unwrap();
    println!("Connected to proxy. Waiting for data");

    // Wait for first packet, then connect to target
    loop {
        let mut data = vec![0; 1024];
        let size = proxy_stream.read(&mut data).await.unwrap();
        if size == 0 {
            println!("Disconnected. Exiting");
            return;
        }
        println!("Proxy responded. Connecting to target");

        let mut target_socket = TcpStream::connect(&args.target).await.unwrap();
        println!("Connected to proxy");
        let (full, _) = data.split_at(size);
        target_socket.write(full).await.unwrap();

        proxy_sockets(&mut proxy_stream, &mut target_socket)
            .await
            .unwrap();
    }
}

async fn deploy(args: DeployArgs) {
    let name = args.name.unwrap_or_else(|| "piggyback".to_string());
    let port = args.port.unwrap_or(8080);
    kubernetes::deploy_proxy(&name, args.namespace, port, args.proxy_image).await;
}

async fn delete(args: DeleteArgs) {
    let name = args.name.unwrap_or_else(|| "piggyback".to_string());
    kubernetes::delete_proxy(&name, args.namespace).await;
}

fn version(_args: VersionArgs) {
    println!("piggyback version {}", env!("GIT_TAG"));
}
