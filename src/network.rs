use std::error::Error;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use futures::future::join_all;
use crate::state::GLOBAL_APP_STATE;
use rmp_serde::{encode, decode};

use crate::message::Message;


pub async fn announce(ip: &str, start_port: u16, end_port: u16) {
    let mut tasks = vec![];

    // lock just to get the local address and site id
    let (local_addr, site_id, local_vc) = {
        let state = GLOBAL_APP_STATE.lock().await;
        (state.get_local_addr().to_string(), state.get_site_id().to_string(), state.get_vector_clock().clone())
    };

    for port in start_port..=end_port {
        let address = format!("{}:{}", ip, port);
        let local_addr_clone = local_addr.clone();
        let site_id_clone = site_id.clone();
        let local_vc = local_vc.clone();

        let task = tokio::spawn(async move {
            let _ = send_message(&address, "", crate::message::NetworkMessageCode::Discovery ,&local_addr_clone, &site_id_clone, &local_vc).await;
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


        let message: Message = decode::from_slice(&buf).expect("Error decoding message");
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        match message.code {
            crate::message::NetworkMessageCode::Discovery => {
                {
                    let mut state = GLOBAL_APP_STATE.lock().await;
                    // envoyer une réponse de découverte
                    println!("Sending discovery response to: {}", message.sender_addr);
                    let ack_code = crate::message::NetworkMessageCode::Acknowledgment;
                    let _ = send_message(&message.sender_addr.to_string(), "", ack_code,&state.get_local_addr(), &state.get_site_id().to_string(),&state.get_vector_clock()).await;
                    println!("DEBUG");
                    // add to list of peers

                    state.add_peer(&message.sender_addr.to_string());
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
                    state.add_peer(&message.sender_addr.to_string());
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
                    state.remove_peer(&message.sender_addr.to_string());
                }
            },
            crate::message::NetworkMessageCode::Sync => {
                println!("Sync message received: {:?}", message);
                on_sync().await;
            },
        }

    }


    let state = GLOBAL_APP_STATE.lock().await;
    let peer_addrs: Vec<SocketAddr> = state.get_peers();
    for peer in &peer_addrs {
        println!("{}", peer);
    }

    Ok(())
}

pub async fn send_message(address: &str, message: &str, code :crate::message::NetworkMessageCode, local_addr:&str, local_site:&str, local_vc :&Vec<u64>) -> Result<(), Box<dyn Error>> {

    // DO NOT LOCK THE GLOBAL APP STATE HERE
    // (or do it at your own risk lol but not my problem anymore)

    let addr = match address.parse::<SocketAddr>() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Failed to parse address {}: {}", address, e);
            return Err(Box::new(e));
        }
    };

    let message = Message {
        sender_id: local_site.parse().unwrap(),
        sender_addr: local_addr.parse().unwrap(),
        sender_vc: local_vc.clone(),
        message: message.to_string(),
        code: code.clone(),
    };

    let buf = encode::to_vec(&message).unwrap();

    let mut stream = TcpStream::connect(addr).await?;
    // add local_addr to the message

    // serialization
    stream.write_all(&buf).await?;
    println!("Sent message {:?} to {}", &message, address);
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
        let local_addr = "127.0.0.1:8080";
        let local_site = "1";
        let local_vc: Vec<u64> = vec![1, 2, 3];

        // Start a listener in a separate task
        tokio::spawn(async move {
            let listener_result = start_listening(address).await;
            assert!(listener_result.is_ok());
        });

        // Give the listener some time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let code = crate::message::NetworkMessageCode::Discovery;

        // Send the message
        let send_result = send_message(address, message,code, local_addr,local_site,&local_vc).await;
        assert!(send_result.is_ok());
    }
}
