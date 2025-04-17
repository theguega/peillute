use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use futures::future::join_all;
use crate::state::GLOBAL_APP_STATE;


pub async fn announce(ip: &str, start_port: u16, end_port: u16) {
    let mut tasks = vec![];

    for port in start_port..=end_port {
        let address = format!("{}:{}", ip, port);
        let message = format!(
            "{}",
            crate::message::NetworkMessageCode::Discovery.code()
        );

        let task = tokio::spawn(async move {
            let _ = send_message(&address, &message).await;
        });

        tasks.push(task);
    }

    join_all(tasks).await;
}


pub async fn start_listening(address: &str) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(address).await?;

    println!("Listening on: {}", address);

    loop {
        let (stream, addr) = listener.accept().await?;

        println!("Accepted connection from: {}", addr);

        if let Err(e) = handle_connection(stream, addr).await {
            eprintln!("Error handling connection from {}: {}", addr, e);
        }
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
        let message = String::from_utf8_lossy(&buf[..n]);
        println!("Message: {}", message);

        // msg format : [site_id]-[local_addr]|[code]
        // for now site_id not used here
        let (site_id, reponse_adress, code) = match message.split_once('|') {
            Some((left, code)) => {
                match left.rsplit_once('-') {
                    Some((site_id, response_adr)) => (site_id.to_string(), response_adr.to_string(), code.to_string()),
                    None => {
                        eprintln!("Malformed message (missing '-')");
                        return Ok(()); 
                    }
                }
            },
            None => {
                eprintln!("Malformed message (missing '|')");
                return Ok(());
            }
        };

        let code = match crate::message::NetworkMessageCode::from_code(&code) {
            Some(c) => c,
            None => {
                eprintln!("Unknown message code: {}", code);
                return Ok(());
            }
        };
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        match code {
            crate::message::NetworkMessageCode::Discovery => {
                println!("New peer connected : {:?}", addr);
                // envoyer une réponse de découverte
                println!("Sending discovery response to: {}", reponse_adress);
                let ack_code = crate::message::NetworkMessageCode::Acknowledgment;
                let _ = send_message(&reponse_adress.to_string(), ack_code.code()).await;
                // add to list of peers
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    state.add_peer(&reponse_adress);
                }
            },
            crate::message::NetworkMessageCode::Transaction => {
                println!("Transaction message received: {:?}", message);
                // handle transaction
            },
            crate::message::NetworkMessageCode::Acknowledgment => {
                println!("Acknowledgment message received: {:?}", message);
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    state.add_peer(&reponse_adress);
                }

            },
            crate::message::NetworkMessageCode::Error => {
                println!("Error message received: {:?}", message);
                // handle error
            },
            crate::message::NetworkMessageCode::Disconnect => {
                println!("Disconnect message received: {:?}", message);
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    state.remove_peer(&reponse_adress);
                }
            },
            crate::message::NetworkMessageCode::Sync => {
                println!("Sync message received: {:?}", message);
                on_sync().await;
            },
            _ => println!("Unknown message code"),
        }

    }

    Ok(())
}

pub async fn send_message(address: &str, message: &str) -> Result<(), Box<dyn Error>> {

    let state = GLOBAL_APP_STATE.lock().await;
    let local_addr = state.get_local_addr();
    let local_site = state.get_site_id();

    let addr = address.parse::<SocketAddr>()?;
    let mut stream = TcpStream::connect(addr).await?;

    // add local_addr to the message
    let message = format!("{}-{}|{}",local_site, local_addr, message);
    stream.write_all(message.as_bytes()).await?;
    println!("Sent '{}' to {}", message, address);

    Ok(())
}


pub async fn on_sync(){
    // TODO : implement sync
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
