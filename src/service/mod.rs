use std::io::{Read, Result};
use std::net::{TcpListener, TcpStream};
use std::thread;

pub struct Service {
    address: String,
}

impl Service {
    pub fn new(port: u16) -> Self {
        Service {
            address: format!("127.0.0.1:{port}"),
        }
    }

    pub fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.address)?;
        println!("Service listening on {}", self.address);

        // this is a blocking call
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(move || {
                        if let Err(e) = Self::handle_connection(stream) {
                            eprintln!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Handles a single connection from a client.
    fn handle_connection(mut stream: TcpStream) -> Result<()> {
        let mut buffer = [0; 1024];

        loop {
            let bytes_read = stream.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // connection closed
            }

            let message = String::from_utf8_lossy(&buffer[..bytes_read]);
            println!("Received message: {}", message);
        }

        Ok(())
    }
}
