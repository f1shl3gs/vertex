use std::convert::Infallible;
use std::net::SocketAddr;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio_util::codec::{Encoder, FramedWrite, LinesCodec};

pub async fn send_lines(
    addr: SocketAddr,
    lines: impl IntoIterator<Item = String>,
) -> Result<(), Infallible> {
    send_encodable(addr, LinesCodec::new(), lines).await
}

pub async fn send_encodable<I, E: From<std::io::Error> + std::fmt::Debug>(
    addr: SocketAddr,
    encoder: impl Encoder<I, Error = E>,
    lines: impl IntoIterator<Item = I>,
) -> Result<(), Infallible> {
    let stream = TcpStream::connect(&addr).await.unwrap();
    let mut sink = FramedWrite::new(stream, encoder);

    let mut lines = futures::stream::iter(lines.into_iter()).map(Ok);
    sink.send_all(&mut lines).await.unwrap();

    let stream = sink.get_mut();
    stream.shutdown().await.unwrap();

    Ok(())
}
