use futures_lite::future;
use tokio::{io, net};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut stream = net::TcpStream::connect("127.0.0.1:6000").await?;
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    println!(
        "[{} -> {}] Type a message and hit enter!\n",
        stream.local_addr()?,
        stream.peer_addr()?
    );

    let (mut reader, mut writer) = stream.split();

    future::race(
        async {
            let res = io::copy(&mut stdin, &mut writer).await;
            println!("Quit!");
            res
        },
        async {
            let res = io::copy(&mut reader, &mut stdout).await;
            println!("Server disconnected!");
            res
        },
    )
    .await?;

    Ok(())
}
