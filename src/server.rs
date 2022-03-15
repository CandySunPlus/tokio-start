use std::{collections::HashMap, net::SocketAddr};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    net, sync,
};

enum Event {
    Join(SocketAddr, net::tcp::OwnedWriteHalf),
    Leave(SocketAddr),
    Message(SocketAddr, String),
}

async fn dispatch(mut receiver: sync::mpsc::Receiver<Event>) -> io::Result<()> {
    let mut map = HashMap::<SocketAddr, net::tcp::OwnedWriteHalf>::new();

    while let Some(event) = receiver.recv().await {
        let (addr, output) = match event {
            Event::Join(addr, stream) => {
                map.insert(addr, stream);
                (addr, format!("{} has joined\n", addr))
            }
            Event::Leave(addr) => {
                map.remove(&addr);
                (addr, format!("{} has left\n", addr))
            }
            Event::Message(addr, message) => (addr, format!("{} says: {}\n", addr, message)),
        };

        print!("{}", output);

        for stream in map.values_mut() {
            if stream.peer_addr()? != addr {
                stream.write_all(output.as_bytes()).await.ok();
            }
        }
    }
    Ok(())
}

async fn read_message(
    sender: sync::mpsc::Sender<Event>,
    client: net::tcp::OwnedReadHalf,
) -> io::Result<()> {
    let addr = client.peer_addr()?;
    let mut lines = io::BufReader::new(client).lines();

    while let Some(line) = lines.next_line().await? {
        sender.send(Event::Message(addr, line)).await.ok();
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    // listen
    // let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 6000))?;
    let listener = net::TcpListener::bind("127.0.0.1:6000").await?;

    let (sender, receiver) = sync::mpsc::channel(100);

    tokio::spawn(dispatch(receiver));

    loop {
        let (stream, addr) = listener.accept().await?;
        let sender = sender.clone();

        tokio::spawn(async move {
            let (rx, tx) = stream.into_split();
            sender.send(Event::Join(addr, tx)).await.ok();
            read_message(sender.clone(), rx).await.ok();
            sender.send(Event::Leave(addr)).await.ok();
        });
    }
}
