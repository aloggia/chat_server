#![allow(unused)]
use async_std::{prelude::*,
task,
net::{TcpListener, ToSocketAddrs, TcpStream},
io::BufReader,
};
use futures::{StreamExt, AsyncBufReadExt, AsyncWriteExt};
use futures::sink::SinkExt;
use futures::{select, FutureExt};
use futures::channel::mpsc;
use std::{collections::hash_map::{Entry, HashMap},
future::Future,
sync::Arc};
use std::option::Option::Some;

fn main() {
    #[derive(Debug)]
    enum Void {}
    #[derive(Debug)]
    enum Event {
        NewPeer {
            name: String,
            stream: Arc<TcpStream>,
            shutdown: Receiver<Void>,
        },
        Message {
            source: String,
            // dest is a vec of strings, because users can send messages to multiple people
            dest: Vec<String>,
            msg: String,
        }
    }

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
    type Sender<T> = mpsc::UnboundedSender<T>;
    type Receiver<T> = mpsc::UnboundedReceiver<T>;

    fn run() -> Result<()> {
        // Put the Servers IP address here, idk what it is yet
        task::block_on(accept_loop())
    }

    async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let (broker_sender, broker_receiver) =mpsc::unbounded();
        let broker_handle = task::spawn(broker_loop(broker_receiver));
        let mut incoming = listener.incoming();
        while let Some(stream) = incoming.next().await {
            // unwrap the stream to extract the stream from the result wrapper
            let stream = stream?;
            //println!("Incoming connection from: {}", stream.peer_addr()?);
            // spawn a new task for each incoming connection
            spawn_log_errors(connection_loop(broker_sender.clone(), stream));
        }
        drop(broker_sender);
        broker_handle.await;
        Ok(())
    }

    async fn connection_loop(mut broker: Sender<Event>, tcp_stream: TcpStream) -> Result<()> {
        let buffer = Arc::new(tcp_stream);
        let reader = BufReader::new(&*tcp_stream);
        let mut lines = buffer.lines();
        // Next 5 lines are for log in, we want to modify them to use the UVMPM log in
        // Name only holds the name of the logged in user, this will need to change
        // TODO: Modify to use the UVMPM log in protocol
        let name = match lines.next().await {
            None => Err("Peer disconected immediatly")?,
            Some(line) => line?,
        };
        let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
        broker.send(Event::NewPeer {
            name: name.clone(),
            stream: Arc::clone(&buffer)
        }).await.unwrap();

        // This while is code to parse messages coming from a client to the server
        //TODO: Modify to only accept messages in the UVMPM format
        while let Some(line) = lines.next().await {
            let line = line?;
            let (destination, msg) = match line.find(':') {
                None => continue,
                Some(index) => (&line[..index], line[index + 1..].trim()),
            };
            let destination: Vec<String> = destination.split(',').map(|name| name.trim().to_string()).collect();
            let msg: String = msg.to_string();

            broker.send(Event::Message {
                source: name.clone(),
                dest: destination,
                msg,
            }).await.unwrap();
        }
        Ok(())
    }

    async fn connection_writer_loop(messages: &mut Receiver<String>,
                                    stream: Arc<TcpStream>,
                                    shutdown: Receiver<Void>) -> Result<()> {
        let mut stream = &*stream;
        let mut messages = messages.fuse();
        let mut shutdown = shutdown.fuse();
        loop {
            select! {
                msg = messages.next().fuse() => match msg {
                    Some(msg) => stream.write_all(msg.as_bytes()).await?,
                    None => break,
                },
                void = shutdown.next().fuse() => match void {
                    Some(void) => match void {},
                    None => break,
                }
            }
        }
        Ok(())
    }

    fn spawn_log_errors<F>(fut: F) -> task::JoinHandle<()>
        where
            F: Future<Output=Result<()>> + Send + 'static,
    {
        task::spawn(async move {
            if let Err(e) = fut.await {
                eprintln!("{}", e)
            }
        })
    }

    async fn broker_loop(events: Receiver<Event>) {
        // Hash map that maps users to destination channels
        let (disconnect_sender, mut disconnect_receiver) =
            mpsc::unbounded::<(String, Receiver<String>)>();
        let mut peers: HashMap<String, Sender<String>> = HashMap::new();
        let mut events = events.fuse();
        loop {
            let event = select! {
                event = events.next().fuse() => match event {
                    None => break,
                    Some(event) => event,
                },
                disconnect = disconnect_receiver.next().fuse() => {
                    let (name, _pending_messages) = disconnect.unwrap();
                    assert!(peers.remove(&name).is_some());
                    continue;
                },
            };
            match event {
                Event::Message { source, dest, msg} => {
                    for addr in dest {
                        if let Some(peer) = peers.get_mut(&addr) {
                            let msg = format!("From:{}:{}\n", source, msg);
                            peer.send(msg).await.unwrap()
                        }
                    }
                }
                Event::NewPeer { name, stream, shutdown } => {
                    match peers.entry(name.clone()) {
                        Entry::Occupied(..) => (),
                        Entry::Vacant(entry) => {
                            let (client_sender, mut client_receiver) = mpsc::unbounded();
                            entry.insert(client_sender);
                            let mut disconnect_sender = disconnect_sender.clone();
                            spawn_log_errors(async move {
                                let res = connection_writer_loop(&mut client_receiver,
                                                                 stream,
                                                                 shutdown).await.unwrap();
                                res
                            });
                        }
                    }
                }
            }
        }
        drop(peers);
        drop(disconnect_sender);
        while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {
        }
    }
}