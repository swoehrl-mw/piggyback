use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn proxy_sockets<A: AsyncRead + AsyncWrite + Unpin, B: AsyncRead + AsyncWrite + Unpin>(
    proxy_socket: &mut A,
    client_socket: &mut B,
) -> Result<(), tokio::io::Error> {
    // Start loop
    let mut data_proxy = vec![0; 1024];
    let mut data_client = vec![0; 1024];
    loop {
        tokio::select! {
            res = proxy_socket.read(&mut data_proxy) => {
                match res {
                    Ok(size) => {
                        if size == 0 {
                            return Ok(());
                        }
                        let (full, _) = data_proxy.split_at_mut(size);
                        client_socket.write(full).await.unwrap();
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            },
            res = client_socket.read(&mut data_client) => {
                match res {
                    Ok(size) => {
                        if size == 0 {
                            return Ok(());
                        }
                        let (full, _) = data_client.split_at_mut(size);
                        proxy_socket.write(full).await.unwrap();
                    },
                    Err(err) => {
                        return Err(err);
                    }
                }
            }
        };
    }
}
