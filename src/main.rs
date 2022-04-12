#![allow(unused)]
use async_std::{prelude::*,
task,
net::{TcpListener, ToSocketAddrs, TcpStream},
io::BufReader,
};
use std::option::Option::Some;
use futures::{StreamExt, AsyncBufReadExt, AsyncWriteExt};
use futures::sink::SinkExt;
use futures::channel::mpsc;
use std::sync::Arc;
use std::collections::hash_map::{Entry, HashMap};

#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>
    },
    Message {
        source: String,
        // dest is a vec of strings, because users can send messages to multiple people
        dest: Vec<String>,
        msg: String,
    }
}


fn main() {

}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

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

async fn connection_writer_loop(mut messages: Receiver<String>, stream: Arc<TcpStream>)
-> Result<()> {
    let mut stream = &*stream;
    while let Some(msg) = messages.next().await {
        stream.write_all(msg.as_bytes()).await?;
    }
    Ok(())
}

fn run() -> Result<()> {
    // Put the Servers IP address here, idk what it is yet
    let fut = accept_loop();
    task::block_on(fut)
}

fn spawn_log_errors<F>(fut: F) -> task::JoinHandle<()>
    where
        F: Future<Output=Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln ! ("{}", e)
        }
    })
}

async fn broker_loop(mut events: Receiver<Event>) -> Result<()> {
    // Hash map that maps users to destination channels
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await {
        match event {
            // Message event, server recieved a message from input, this forwards it to the correct client
            Event::Message { source, dest, msg} => {
                for addr in dest {
                    if let Some(peer) = peers.get_mut(&addr) {
                        // Properly formatted in UVMPM protocol sending style
                        let msg = format!("From:{}:{}\n", source, msg);
                        peer.send(msg).await?
                    }
                }
            }
            // New user has joined the server, we need to add them to the hash map of clients connected
            // to the server
            Event::NewPeer { name, stream} => {
                match peers.entry(name) {
                    // user is already in the hash map
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        // This task will write any messages to the new peers socket
                        spawn_log_errors(connection_writer_loop(client_receiver, stream));
                    }
                }
            }
        }
    }


    Ok(())
}