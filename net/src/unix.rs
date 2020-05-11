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

    pub fn into_incoming(
        self,
    ) -> impl futures::stream::Stream<Item = Result<Stream, std::io::Error>> {
        futures::stream::iter(vec![Ok(self)].into_iter().map(|v| {
            debug!("iter: get new uds-stream");
            v
        }))
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
    guard: Option<FdGuard>,
}

impl Listener {
    // pub fn should_close_on_drop(mut self, value: bool) -> Self {
    //     self.should_close_on_drop = value;
    //     self
    // }

    pub fn bind<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let listener = UnixListener::bind(path)?;
        let guard = FdGuard {
            enabled: true,
            path: listener.local_addr()?,
        };
        Ok(Self {
            inner: listener,
            guard: Some(guard),
        })
    }

    pub fn incoming(
        &mut self,
    ) -> impl futures::stream::Stream<Item = Result<Stream, std::io::Error>> + '_ {
        use futures::stream::TryStreamExt;
        self.inner.incoming().map_ok(Stream::new)
    }

    /// Builder-pattern-like enable or disable inner fd-guard.
    /// If disable prevent unlink (kill, close) the socket on drop.
    pub fn guarded(mut self, enabled: bool) -> Self {
        if let Some(guard) = &mut self.guard {
            guard.enabled = enabled;
        }
        self
    }

    /// Enable or disable inner fd-guard. If disable prevent unlink (kill, close) the socket on drop.
    pub fn set_guard(&mut self, enabled: bool) {
        if let Some(guard) = &mut self.guard {
            guard.enabled = enabled;
        }
    }

    /// Take and return the guard. Return `None` if already taken.
    pub fn guard(&mut self) -> Option<FdGuard> {
        self.guard.take()
    }
}

#[derive(Debug)]
pub struct FdGuard {
    enabled: bool,
    path: std::os::unix::net::SocketAddr,
}

impl Drop for FdGuard {
    fn drop(&mut self) {
        debug!("UDS fd-guard dropping");
        println!("UDS fd-guard dropping");

        if !self.enabled {
            debug!("UDS fd-guard dropping skipped.");
            println!("UDS fd-guard dropping skipped.");
            return;
        }

        if let Some(path) = self.path.as_pathname() {
            match unlink_uds(path) {
                Ok(_) => debug!("UDS fd closed"),
                Err(err) => error!("{}", err),
            }
        } else {
            error!("Failed to close UDS fd: No local pathname.");
        }
    }
}

// impl Drop for Listener {
//     fn drop(&mut self) {
//         debug!("UDS listener dropping");

//         match self.inner.local_addr() {
//             Ok(addr) => {
//                 if let Some(path) = addr.as_pathname() {
//                     match unlink_uds(path) {
//                         Ok(_) => debug!("UDS fd closed"),
//                         Err(err) => error!("{}", err),
//                     }
//                 } else {
//                     error!("Failed to close UDS fd: No local pathname.");
//                 }
//             }
//             Err(err) => error!("Failed to close UDS fd: {}", err),
//         }
//     }
// }

pub fn unlink_uds<P: AsRef<Path>>(path: P) -> Result<(), std::io::Error> {
    use std::io::{Error, ErrorKind};
    use std::process::Command;
    use std::str::from_utf8;

    if let Some(path) = path.as_ref().to_str() {
        match Command::new("unlink").arg(path).output() {
            Ok(output) => {
                if output.status.success() {
                    Ok(())
                } else {
                    let err = from_utf8(&output.stderr).ok().unwrap_or("unknown error");
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
            "Failed to close UDS channel: invalid path",
        ))
    }
}
