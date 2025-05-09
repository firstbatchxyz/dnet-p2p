use eyre::Context;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_util::sync::CancellationToken;

/// Listen on all interfaces on a random port.
const LISTEN_ADDR: &str = "0.0.0.0:0";

pub struct DnetService {
    cancellation: CancellationToken,
    listener: TcpListener,
}

impl DnetService {
    pub async fn new(cancellation: CancellationToken) -> eyre::Result<Self> {
        Ok(Self {
            cancellation,
            listener: TcpListener::bind(LISTEN_ADDR).await?,
        })
    }

    /// Returns the local port that the service is listening on.
    pub fn get_port(&self) -> eyre::Result<u16> {
        self.listener
            .local_addr()
            .map(|a| a.port())
            .wrap_err("could not get local address")
    }

    pub async fn start(&self) -> eyre::Result<()> {
        // let client = TcpStream::connect(&addr).await?;

        loop {
            tokio::select! {
              // handle incoming connections
              accept_result = self.listener.accept() => {
                match accept_result {
                  Ok((connection, _)) => {
                    if let Err(e) = self.handle_connection(connection).await {
                      log::error!("Failed to handle connection: {e}");
                    }
                  }
                  Err(err) => {
                    log::error!("Failed to accept connection: {err}");
                  }
                }
              }

              // FIXME: client handling logic here


              _ = self.cancellation.cancelled() => break,
            }
        }

        Ok(())
    }

    async fn handle_connection(&self, mut connection: TcpStream) -> eyre::Result<()> {
        log::info!("Accepted connection from {}", connection.peer_addr()?);

        // read the response
        connection.readable().await?;
        let mut buffer = vec![0; 1024];
        let n = connection.try_read(&mut buffer)?;
        if n == 0 {
            log::warn!("Connection closed by peer");
            return Ok(());
        }
        let data = &buffer[..n];
        log::info!("Received data: {:?}", String::from_utf8_lossy(data));

        // send a response back
        connection.writable().await?;
        connection.write_all(b"PONG").await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    use super::*;

    // cargo test --package dnet-p2p --lib -- service::core::tests::test_send_dummy --exact --show-output
    #[tokio::test]
    async fn test_send_dummy() -> eyre::Result<()> {
        let port = 57421;
        // let addr = format!("127.0.0.1:{port}");
        let addr = format!("192.168.1.119:{port}");

        let mut client = TcpStream::connect(&addr).await?;

        // send hello world
        let msg = b"Hello, world!";
        client.write_all(msg).await?;

        Ok(())
    }
}
