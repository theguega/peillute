use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn start_listening(address: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(address).await?;

    println!("Listening on: {}", address);

    loop {
        let (stream, addr) = listener.accept().await?;

        println!("Accepted connection from: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, addr).await {
                eprintln!("Error handling connection from {}: {}", addr, e);
            }
        });
    }
}

pub async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 1024];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            println!("Connection closed by: {}", addr);
            break;
        }

        println!("Received {} bytes from {}", n, addr);
        println!("Message: {}", String::from_utf8_lossy(&buf[..n]));
    }

    Ok(())
}

pub async fn send_message(address: &str, message: &str) -> Result<(), Box<dyn Error>> {
    let addr = address.parse::<SocketAddr>()?;

    let mut stream = TcpStream::connect(addr).await?;

    stream.write_all(message.as_bytes()).await?;
    println!("Sent '{}' to {}", message, address);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_send_message() {
        let address = "127.0.0.1:8081";
        let message = "hello";

        // Start a listener in a separate task
        tokio::spawn(async move {
            let listener_result = start_listening(address).await;
            assert!(listener_result.is_ok());
        });

        // Give the listener some time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send the message
        let send_result = send_message(address, message).await;
        assert!(send_result.is_ok());
    }
}
