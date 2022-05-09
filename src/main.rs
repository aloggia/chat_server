#![allow(unused)]

// Chat server structure is copied from: https://book.async.rs/tutorial/handling_disconnection.html
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
    env,
};
use std::io::Error;

fn main() {
    run();
}
// These are all channels to send messages between threads
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[derive(Debug)]
enum Void {}

/*
There are 2 different things that can happen while the server is running, a user can connect
and a user can send a message, this enum allows for pattern matching to both events
 */
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
    // Put the Servers IP address here
    let args: Vec<String> = env::args().collect();
    let address = format!("{}:{}", args[1], args[2]);
    println!("{:?}", address);
    // Accept loop run in it's own thread
    task::block_on(accept_loop(address))
}

/*
accept_loop is listening on a port for incoming connection requests
when it recieves a request it first calls the spawn_log_errors function
to run in the background and gracefully catch any errors
It then calls the connection loop which lets the server read incoming data from spawned sockets
 */
async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    // listen to a tcp socket for a new connection
    let listener = TcpListener::bind(addr).await?;
    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker_handle = task::spawn(broker_loop(broker_receiver));
    let mut incoming = listener.incoming();
    // Socket heard a new incoming connection
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
    /*
    Handshake logic. In an ideal world I would have the server send a handshake response
    But idk how to send a message directly to the client from the server
     */
    let mut correct_handshake = false;
    while !correct_handshake {
        let handshake = match lines.next().await {
            None => Err("No handshake")?,
            Some(line) => { line? }
        };
        println!("{}", handshake);
        if handshake == "HELLO" {
            correct_handshake = true;
        }
        println!("{:?}", correct_handshake);
    }
    println!("Handshake successful");
    // Don't think I can send a message from server to client without the client being in the broker table
    // Add the user to the broker table, to allow the user to log in
    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };
    broker.send(Event::NewPeer {
        name: name.clone(),
        stream: Arc::clone(&stream),
        shutdown: shutdown_receiver,
    }).await.unwrap();
    let mut user_authenticated: bool = false;
    // switch statement for cases: register, check_user, log_in
    // Rather then pass true/false through pipes, the python program prints 0 if it did it's job
    // successfully, or a 1 if it didn't do it's job successfully
    while !user_authenticated {
        // line will be in format pythonRequest:username:password
        let username_passwd_incoming = match lines.next().await {
            None => Err("User didn't send username & password")?,
            Some(line) => line?,
        };
        let mut split = username_passwd_incoming.split(":");
        let username_password: Vec<&str> = split.collect();
        let mut check_name_output;
        match username_password[0] {
            "register" => {
                // run the python program
                check_name_output = Command::new("python")
                    .arg("userDatabase/main.py")
                    .arg("register")
                    .arg(username_password[1])
                    .arg(username_password[2])
                    .output()
                    .expect("Failed to register user");
                println!("{:?}", String::from_utf8(check_name_output.stdout.clone()).unwrap());
                // check it's output
                if String::from_utf8(check_name_output.stdout.clone()).unwrap() == "0\n" {
                    user_authenticated = false;
                } else {
                    user_authenticated = true;
                }
                println!("{:?}", user_authenticated);
            }
            "check_user" => {
                // run python program
                check_name_output = Command::new("python")
                    .arg("userDatabase/main.py")
                    .arg("check_user")
                    .arg(username_password[1])
                    .arg(username_password[2])
                    .output()
                    .expect("Failed to check user");
                println!("{:?}", String::from_utf8(check_name_output.stdout.clone()).unwrap());
                // Check python output
                if String::from_utf8(check_name_output.stdout.clone()).unwrap() == "0\n" {
                    user_authenticated = false;
                } else {
                    user_authenticated = true;
                }
                println!("{:?}", user_authenticated);
            }
            "log_in" => {
                // Run python program
                check_name_output = Command::new("python")
                    .arg("userDatabase/main.py")
                    .arg("log_in")
                    .arg(username_password[1])
                    .arg(username_password[2])
                    .output()
                    .expect("Failed to log user in");
                println!("{:?}", String::from_utf8(check_name_output.stdout.clone()).unwrap());
                // Check it's output
                if String::from_utf8(check_name_output.stdout.clone()).unwrap() == "0\n" {
                    user_authenticated = false;
                } else {
                    user_authenticated = true;
                }
                println!("{:?}", user_authenticated);
            }
            // Default base case
            &_ => { println!("Reached default database access case") }
        }
    }
    println!("Left switch statement");
    /*
    Message logic, wait for the client to send a message, then process it when recieved
     */
    while let Some(line) = lines.next().await {
        // Unwrap line
        let line = line?;
        // split the message into destination(s) and message body
        let (dest, msg) = match line.find(':') {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        };
        // capture the destination(s) in a vector with a closure
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();
        // Send the message to the broker
        broker.send(Event::Message {
            source: name.clone(),
            dest,
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
    // Will loop until there are no more messages to send, and select! chooses shutdown
    loop {
        select! {
            // Send messages in queue
                msg = messages.next().fuse() => match msg {
                    Some(msg) => stream.write_all(msg.as_bytes()).await?,
                    None => break,
                },
            // Shutdown the connection
                void = shutdown.next().fuse() => match void {
                    Some(void) => match void {},
                    None => break,
                }
            }
    }
    Ok(())
}
// Error catching bucket, prints all errors to console rather than crash program
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
    // Loops until there are no more events
    loop {
        let event = select! {
            // Either do the next event in the events queue, or close the connection
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
            // If the next event is a message, look up the destination channels in the hash table
            // and send the message out through the destination channels
            Event::Message { source, dest, msg } => {
                for addr in dest {
                    if let Some(peer) = peers.get_mut(&addr) {
                        // Formatted to use UVMPM protocol
                        let msg = format!("From:{}:{}\n", source, msg);
                        peer.send(msg).await.unwrap()
                    }
                }
            }
            // If the next event is a new user, add them to the hashmap
            Event::NewPeer { name, stream, shutdown } => {
                match peers.entry(name.clone()) {
                    // If user is already logged on, do nothing
                    Entry::Occupied(..) => (),
                    // Otherwise hash them into the hashtable
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
    // When there are no more events, all the peers will be dropped and the server will shutdown
    drop(peers);
    drop(disconnect_sender);
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {}
}