extern crate http;

use std::{
    convert::{TryInto, TryFrom},
    path::Path,
};
use std::io::{Error as IoError, ErrorKind};
use http::Uri;
use crate::tonic;
use tonic::transport::Channel;

#[derive(Debug, Clone)]
pub enum Endpoint {
    Http(Http),
    Ipc(Ipc),
}

impl Endpoint {
    pub async fn connect(self) -> Result<Channel, Box<dyn std::error::Error>> {
        match self {
            Endpoint::Http(inner) => futures::future::Either::Left(inner.connect()),
            Endpoint::Ipc(inner) => futures::future::Either::Right(inner.connect()),
        }
        .await
    }

    pub fn is_ipc(&self) -> bool {
        use Endpoint::*;
        match self {
            Ipc(_) => true,
            Http(_) => false,
        }
    }

    pub fn is_http(&self) -> bool {
        use Endpoint::*;
        match self {
            Ipc(_) => false,
            Http(_) => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Http(pub(crate) std::net::SocketAddr);
#[derive(Debug, Clone)]
pub struct Ipc(pub(crate) std::path::PathBuf);
// std::os::unix::net::SocketAddr

impl Http {
    /// Connect using default transport (http2).
    pub async fn connect(self) -> Result<Channel, Box<dyn std::error::Error>> {
        trace!("connecting http {:?}", &self.0);
        let uri = format!("http://{}", self.0.to_string());
        let conn = tonic::transport::Endpoint::new(uri)?.connect().await?;
        trace!("connected http {:?}", &self.0);
        Ok(conn)
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Ipc {
    /// Create all dirs for path if not exists.
    pub(crate) async fn create_dir_all(path: &Path) -> Result<(), std::io::Error> {
        trace!("preparing ipc fs-path {:?}", &path);
        tokio::fs::create_dir_all(path.parent().unwrap_or_else(|| path)).await?;
        Ok(())
    }

    /// Connect using UDS transport.
    pub async fn connect(self) -> Result<Channel, Box<dyn std::error::Error>> {
        use tonic::transport::Endpoint;
        use crate::transport::Stream;
        use tower::service_fn;

        trace!("connecting ipc {:?}", &self.0);
        // Here magic: we need to trigger fallback, so passing valid uri but unsupported
        let channel = Endpoint::try_from("ipc://dummy")?
            .connect_with_connector(service_fn(move |_: Uri| Stream::connect(self.0.clone())))
            .await?;
        trace!("connected ipc");
        Ok(channel)
    }

    pub fn as_str(&self) -> Option<&str> {
        self.0.to_str()
    }
}

impl std::str::FromStr for Endpoint {
    type Err = crate::StdError;

    /// Create Endpoint from URI.
    /// Supports http and ipc schemes.
    ///
    /// e.g.:
    /// - `ipc://tmp/dir/file` (absolute path)
    /// - `ipc://./dir/file` (relative path with `.` and `..`)
    /// - `ipc://~/dir/file` (relative to $HOME)
    /// - `http://[::1]:50042`
    /// - `http://127.0.0.1:50042`
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        from_uri(s.parse()?)
    }
}

impl TryFrom<Uri> for Endpoint {
    type Error = crate::StdError;

    fn try_from(uri: Uri) -> Result<Self, Self::Error> {
        from_uri(uri)
    }
}

impl<'a> TryInto<&'a Path> for &'a Endpoint {
    // type Error = crate::StdError;
    type Error = std::io::Error;

    fn try_into(self) -> Result<&'a Path, Self::Error> {
        match self {
            Endpoint::Ipc(Ipc(pb)) => Ok(pb.as_path()),
            _ => Err(IoError::new(
                ErrorKind::Other,
                "Only IPC can `into` `Path`.",
            )),
        }
    }
}

pub fn from_uri(uri: Uri) -> Result<Endpoint, crate::StdError> {
    let parts = uri.into_parts();

    if let Some(scheme) = parts.scheme {
        let mut addr = parts
            .authority
            .as_ref()
            .map(|a| a.as_str())
            .unwrap_or("")
            .to_owned();
        if let Some(path) = parts.path_and_query.as_ref() {
            match path.as_str() {
                "" | "/" => { /* empty */ }
                s => addr.push_str(s),
            }
        }

        match scheme.as_str() {
            "http" => Ok(Endpoint::Http(Http(addr.parse()?))),
            "ipc" | "uds" => {
                match addr.chars().next() {
                    Some('.') | Some('~') => { /* relative path */ }
                    Some('/') => { /* absolute path */ }
                    _ => addr = "/".to_owned() + &addr,
                }
                Ok(Endpoint::Ipc(Ipc(addr.parse()?)))
            }
            _ => Err(IoError::new(
                ErrorKind::Other,
                format!("Protocol {} not supported", scheme.as_str()),
            )
            .into()),
        }
    } else {
        Err(IoError::new(ErrorKind::Other, "Protocol is missed").into())
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Endpoint::Http(Http(endpoint)) => write!(f, "{}", endpoint.to_string()),
            Endpoint::Ipc(Ipc(endpoint)) => write!(f, "{}", endpoint.to_string_lossy()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn from_str_absolete_ipc() {
        let endpoint: Result<Endpoint, _> = "ipc://tmp/dir/file".parse();
        assert!(endpoint.is_ok());

        if let Ok(endpoint) = endpoint {
            match endpoint {
                Endpoint::Http(_) => panic!("expected IPC"),
                Endpoint::Ipc(inner) => {
                    const INV_PATH: &str = "invalid path";
                    let path: PathBuf = inner.as_str().expect(INV_PATH).parse().expect(INV_PATH);
                    assert!(path.is_absolute());
                }
            }
        } else {
            unreachable!();
        }
    }

    #[test]
    fn from_str_relative_ipc() {
        let paths = ["ipc://./tmp/file", "ipc://../tmp/file"];
        for path in paths.iter() {
            let endpoint: Result<Endpoint, _> = path.parse();
            assert!(endpoint.is_ok());

            if let Ok(endpoint) = endpoint {
                match endpoint {
                    Endpoint::Http(_) => panic!("expected IPC"),
                    Endpoint::Ipc(inner) => {
                        const INV_PATH: &str = "invalid path";
                        let path: PathBuf =
                            inner.as_str().expect(INV_PATH).parse().expect(INV_PATH);
                        assert!(path.is_relative());
                    }
                }
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn from_str_http() {
        let expected = ["[::1]:50042", "127.0.0.1:50042"];
        let uris = ["http://[::1]:50042", "http://127.0.0.1:50042"];

        for (i, uri) in uris.iter().enumerate() {
            let endpoint: Result<Endpoint, _> = uri.parse();
            assert!(endpoint.is_ok());

            if let Ok(endpoint) = endpoint {
                match endpoint {
                    Endpoint::Http(inner) => {
                        assert_eq!(expected[i], &inner.to_string());
                    }
                    Endpoint::Ipc(_) => panic!("expected HTTP"),
                }
            } else {
                unreachable!();
            }
        }
    }
}
