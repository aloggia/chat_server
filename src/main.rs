#![allow(unused)]

use async_std::{
    io::BufReader,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    task,
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::{select, FutureExt, StreamExt};
use std::{
    collections::hash_map::{Entry, HashMap},
    future::Future,
    sync::Arc,
    time::{Duration, Instant},
    process::Command,
};
use std::io::Error;

fn main() {
    run();
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

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
    },
}

fn run() -> Result<()> {
    // Put the Servers IP address here, idk what it is yet
    task::block_on(accept_loop("192.168.1.195:8080"))
}

/*
accept_loop is listening on a port for incoming connection requests
when it recieves a request it first calls the spawn_log_errors function
to run in the background and gracefully catch any errors
It then calls the connection loop which lets the server read incoming data from spawned sockets
 */
async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker_handle = task::spawn(broker_loop(broker_receiver));
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        // unwrap the stream to extract the stream from the result wrapper
        let stream = stream?;
        println!("Incoming connection from: {}", stream.peer_addr()?);
        // spawn a new task for each incoming connection
        spawn_log_errors(connection_loop(broker_sender.clone(), stream));
    }
    drop(broker_sender);
    broker_handle.await;
    Ok(())
}

/*
Listen to active sockets, data is read in through TcpStream, and is put in an Arc, which
is a thread safe wrapper that allows a shared pointer to a single resource
Reader is a buffer holding all the data that is read from the tcp stream
connection_loop is a listener, when it detects a new user it sends that users info in an enum
to broker loop, then it enters the listening loop where it waits for new messages from the client
broker is a channel to the broker_loop, so send any data that needs to get sent/added to hash_map
through the broker
 */
async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
    let mut lines = reader.lines();
    // TODO: Add handshake logic here
    // TODO: Add a timeout -> If no handshake recieved in a unit of time, then close connection
    /*
    idea: start timer
    if time >= timeout_length { Ok(()) }
     */
    let handshake = match lines.next().await {
        None => Err("No handshake")?,
        Some(line) => {line?},
    };
    println!("{}", handshake);
    if handshake != "HELLO" {
        Err("Incorrect Handshake")?;
    } else if handshake == "HELLO" {
        // Don't think I can send a message from server to client without the client being in the broker table
        let name = match lines.next().await {
            None => Err("peer disconnected immediately")?,
            Some(line) => line?,
        };
        broker.send(Event::NewPeer {
            name: name.clone(),
            stream: Arc::clone(&stream),
            shutdown: shutdown_receiver,
        }).await.unwrap();
        // TODO: switch statment for cases: register, check_user, log_in
        let username_passwd_incoming = match lines.next().await {
            None => Err("User didn't send username & password")?,
            Some(line) => line?,
        };
        let mut split = username_passwd_incoming.split(":");
        let username_password: Vec<&str> = split.collect();
        println!("{:?}", username_password);
        let mut check_name_output;
        check_name_output = Command::new("python")
            .arg("userDatabase/main.py")
            .arg("check_user")
            .arg(username_password[0])
            .arg(username_password[1])
            .output()
            .expect("Check username failed");
        println!("{:?}", check_name_output.stdout);
        println!("{:?}", String::from_utf8(check_name_output.stdout.clone()).unwrap());
        if String::from_utf8(check_name_output.stdout.clone()).unwrap() == "0\n" {
            println!("Disconnecting, not in system");
            return Ok(())
        } else {
            let mut password = match lines.next().await {
                None => Err("No password sent from user")?,
                Some(line) => line?,
            };
            println!("{:?}", &password);
            let mut enter_password_output = Command::new("python")
                .arg("userDatabase/main.py")
                .arg("log_in")
                .arg(&name)
                .output()
                .expect("Check password failed");
            println!("{:?}", String::from_utf8(enter_password_output.stdout.clone()).unwrap());
            while String::from_utf8(enter_password_output.stdout.clone()).unwrap() == "0\n" {
                // Password is wrong, prompt user for new password
                broker.send(Event::Message {
                    source: stream.local_addr().unwrap().to_string(),
                    dest: vec![name.clone()],
                    msg: "LOGINFAIL".parse().unwrap()
                });
                let username_passwd_incoming = match lines.next().await {
                    None => Err("User didn't send username & password")?,
                    Some(line) => line?,
                };
                let mut split = username_passwd_incoming.split(":");
                let username_password: Vec<&str> = split.collect();
                enter_password_output = Command::new("python")
                    .arg("userDatabase/main.py")
                    .arg("log_in")
                    .arg(username_password[0])
                    .arg(username_password[1])
                    .output()
                    .expect("Check password failed");
                println!("{:?}", String::from_utf8(enter_password_output.stdout.clone()).unwrap());
            }

        }
        while let Some(line) = lines.next().await {
            let line = line?;
            let (dest, msg) = match line.find(':') {
                None => continue,
                Some(idx) => (&line[..idx], line[idx + 1..].trim()),
            };
            let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
            let msg: String = msg.trim().to_string();
            broker.send(Event::Message {
                source: name.clone(),
                dest,
                msg,
            }).await.unwrap();
        }

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
            Event::Message { source, dest, msg } => {
                for addr in dest {
                    if let Some(peer) = peers.get_mut(&addr) {
                        // Formatted to use UVMPM protocol
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
                                                             shutdown).await;
                            disconnect_sender.send((name, client_receiver)).await.unwrap();
                            res
                        });
                    }
                }
            }
        }
    }
    drop(peers);
    drop(disconnect_sender);
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {}
}

/*
let handshake = match lines.next().await {
    None => Err("Incomplete handshake")?,
    Some(handshake) => {
        // Client sent the wrong message as the first part of the handshake
        if line? != "HELLO\n" {
            Err("Incorrect handshake")?;
        } else if line? == "HELLO\n" {
            broker.send(Event::Message {
                source: stream.local_addr().unwrap().to_string(),
                dest: vec![stream.peer_addr().unwrap().to_string()],
                msg: "HELLO\n".parse().unwrap(),
            });
        }
    }
    _ => {}
};
*/