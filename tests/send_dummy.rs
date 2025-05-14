use tokio::{io::AsyncWriteExt, net::TcpStream};

// cargo test --package dnet-p2p --test send_dummy -- test_send_dummy --exact --show-output
#[tokio::test]
#[ignore = "requires a running service"]
async fn test_send_dummy() -> eyre::Result<()> {
    let port = 57421;
    let addr = format!("192.168.1.119:{port}");

    let mut client = TcpStream::connect(&addr).await?;

    // send hello world
    let msg = b"Hello, world!";
    client.write_all(msg).await?;

    Ok(())
}
