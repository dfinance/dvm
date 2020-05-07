use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::net::UnixStream;
use tokio::net::UnixListener;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::tonic;
use tonic::transport::server::Connected;

#[derive(Debug)]
pub struct Stream(UnixStream);

impl Stream {
    pub fn new(stream: UnixStream) -> Self {
        Self(stream)
    }

    pub async fn connect<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        Ok(Self(UnixStream::connect(path).await?))
    }
}

impl Connected for Stream {}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

#[derive(Debug)]
pub struct Listener {
    inner: UnixListener,
    should_close_on_drop: bool,
}

impl Listener {
    pub fn should_close_on_drop(mut self, value: bool) -> Self {
        self.should_close_on_drop = value;
        self
    }

    pub fn bind<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        Ok(Self {
            inner: UnixListener::bind(path)?,
            should_close_on_drop: false,
        })
    }

    pub fn incoming(
        &mut self,
    ) -> impl futures::stream::Stream<Item = Result<Stream, std::io::Error>> + '_ {
        use futures::stream::TryStreamExt;
        self.inner.incoming().map_ok(Stream::new)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        // TODO: debug
        println!("Listener DROPPING");

        if let Ok(addr) = self.inner.local_addr() {
            if let Some(path) = addr.as_pathname() {
                match unlink_uds(path) {
                    // TODO: debug, warn
                    Ok(_) => println!("UDS channel closed"),
                    Err(err) => eprintln!("{}", err),
                }
            }
        } else {
            eprint!("Failed to close UDS channel");
        }
    }
}

pub fn unlink_uds<P: AsRef<Path>>(path: P) -> Result<(), std::io::Error> {
    use std::io::{Error, ErrorKind};
    use std::process::Command;

    if let Some(path) = path.as_ref().to_str() {
        match Command::new("unlink").arg(path).output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let err = std::str::from_utf8(&output.stderr)
                        .ok()
                        .unwrap_or("unknown error");
                    Err(Error::new(
                        ErrorKind::Other,
                        format!(
                            "Failed to close UDS channel: ({:?}) {}",
                            output.status.code(),
                            err
                        ),
                    ))
                }
            }
            Err(err) => Err(Error::new(
                ErrorKind::Other,
                format!("Failed to close UDS channel: {}", err),
            )),
        }
    } else {
        Err(Error::new(
            ErrorKind::Other,
            format!("Failed to close UDS channel: invalid path"),
        ))
    }
}
