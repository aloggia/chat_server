use async_std::{prelude::*,
task,
net::{TcpListener, ToSocketAddrs, TcpStream},
io::BufReader,
};
use std::option::Option::Some;
use futures::{StreamExt, AsyncBufReadExt};


fn main() {

}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        // unwrap the stream to extract the stream from the result wrapper
        let stream = stream?;
        //println!("Incoming connection from: {}", stream.peer_addr()?);
        // spawn a new task for each incoming connection
        let _handle = task::spawn(connection_loop(stream));
    }
    Ok(())
}

async fn connection_loop(tcp_stream: TcpStream) -> Result<()> {
    let buffer = BufReader::new(&tcp_stream);
    let mut lines = buffer.lines();
    // Next 5 lines are for log in, we want to modify them to use the UVMPM log in
    // Name only holds the name of the logged in user, this will need to change
    // TODO: Modify to use the UVMPM log in protocol
    let name = match lines.next().await {
        None => Err("Peer disconected immediatly")?,
        Some(line) => line?,
    };
    println!("conected too {}", name);

    // This while is code to parse messages coming from a client to the server
    //TODO: Modify the incoming message syntax to use the UVMPM protocol
    while let Some(line) = lines.next().await {
        let line = line?;
        let (destination, msg) = match line.find(':') {
            None => continue,
            Some(index) => (&line[..index], line[index + 1 ..].trim()),
        };
        let destination: Vec<String> = destination.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.to_string();
    }

    Ok(())

}

fn run() -> Result<()> {
    // Put the Servers IP address here, idk what it is yet
    let fut = accept_loop();
    task::block_on(fut)
}