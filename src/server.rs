use std::{
    collections::HashMap,
    net::{SocketAddr, TcpListener, TcpStream},
};

use async_dup::Arc;
use smol::{
    channel::{bounded, Receiver, Sender},
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    stream::StreamExt,
    Async,
};

enum Event {
    Join(SocketAddr, Arc<Async<TcpStream>>),
    Leave(SocketAddr),
    Message(SocketAddr, String),
}

async fn dispatch(receiver: Receiver<Event>) -> io::Result<()> {
    let mut map = HashMap::<SocketAddr, Arc<Async<TcpStream>>>::new();

    while let Ok(event) = receiver.recv().await {
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
            if stream.get_ref().peer_addr()? != addr {
                stream.write_all(output.as_bytes()).await.ok();
            }
        }
    }
    Ok(())
}

async fn read_message(sender: Sender<Event>, client: Arc<Async<TcpStream>>) -> io::Result<()> {
    let addr = client.get_ref().peer_addr()?;
    let mut lines = io::BufReader::new(client).lines();

    while let Some(line) = lines.next().await {
        let line = line?;
        sender.send(Event::Message(addr, line)).await.ok();
    }
    Ok(())
}

fn main() -> io::Result<()> {
    smol::block_on(async {
        // listen
        let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 6000))?;

        let (sender, receiver) = bounded(100);

        smol::spawn(dispatch(receiver)).detach();

        loop {
            let (stream, addr) = listener.accept().await?;
            let client = Arc::new(stream);
            let sender = sender.clone();

            smol::spawn(async move {
                sender.send(Event::Join(addr, client.clone())).await.ok();

                read_message(sender.clone(), client).await.ok();

                sender.send(Event::Leave(addr)).await.ok();
            })
            .detach();
        }
    })
}
