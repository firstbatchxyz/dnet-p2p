use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio_util::sync::CancellationToken;

pub struct DnetService {
    port: u16,
    cancellation: CancellationToken,
}

impl DnetService {
    pub fn new(port: u16, cancellation: CancellationToken) -> Self {
        Self { port, cancellation }
    }

    pub async fn start(&self) -> eyre::Result<()> {
        // TODO: use a better address, this may not be that secure maybe?
        let addr = format!("0.0.0.0:{}", self.port);

        let listener = TcpListener::bind(&addr).await?;
        // let client = TcpStream::connect(&addr).await?;

        loop {
            select! {
              // handle incoming connections
              accept_result = listener.accept() => {
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

    async fn handle_connection(&self, connection: TcpStream) -> eyre::Result<()> {
        log::info!("Accepted connection from {}", connection.peer_addr()?);

        // wait for data to be available
        connection.readable().await?;

        let mut buffer = vec![0; 1024];
        let n = connection.try_read(&mut buffer)?;
        if n == 0 {
            log::warn!("Connection closed by peer");
            return Ok(());
        }

        // process data
        let data = &buffer[..n];
        log::info!("Received data: {:?}", String::from_utf8_lossy(data));

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
        let port = 5678;
        // let addr = format!("127.0.0.1:{port}");
        let addr = format!("192.168.1.119:{port}");

        let mut client = TcpStream::connect(&addr).await?;

        // send hello world
        let msg = b"Hello, world!";
        client.write_all(msg).await?;

        Ok(())
    }
}
