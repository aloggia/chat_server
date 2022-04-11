use async_std::{prelude::*,
task,
net::{TcpListener, ToSocketAddrs}};
use std::option::Option::Some;
use futures::StreamExt;


fn main() {
    let listener: std::net::TcpListener = unimplemented!();
    for stream in listener.incoming() {
        //TODO
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(stream) = incoming.next().await {
        //TODO
    }
    Ok(())
}