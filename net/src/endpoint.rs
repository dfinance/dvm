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

/// Create Endpoint with inner URI.
/// Supports http and https.
/// Can `TryInto` into `std::net::SocketAddr`
/// without IP resolving.
///
/// e.g.:
/// - `http://[::1]:50042`
/// - `http://127.0.0.1:50042`
/// - `http://example.com:50042`
#[derive(Debug, Clone)]
pub struct Http(pub(crate) Uri);
#[derive(Debug, Clone)]
pub struct Ipc(pub(crate) std::path::PathBuf);

impl Http {
    /// Connect using default transport (http2).
    pub async fn connect(self) -> Result<Channel, Box<dyn std::error::Error>> {
        trace!("connecting tcp/ip {:?}", &self.0);
        let conn = tonic::transport::Endpoint::new(self.0.to_string())?
            .connect()
            .await?;
        trace!("connected tcp/ip {:?}", &self.0);
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

impl TryInto<std::net::SocketAddr> for Http {
    type Error = std::io::Error;
    fn try_into(self) -> Result<std::net::SocketAddr, Self::Error> {
        let uri = self.0;
        let parts = uri.into_parts();
        if let Some(scheme) = parts.scheme {
            debug_assert!(scheme.as_str().starts_with("http"));

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

            addr.parse()
                .map_err(|err| IoError::new(ErrorKind::Other, format!("{}", err)))
        } else {
            Err(IoError::new(
                ErrorKind::InvalidInput,
                "Scheme missed in the uri.",
            ))
        }
    }
}

impl TryInto<std::net::SocketAddr> for Endpoint {
    type Error = std::io::Error;
    fn try_into(self) -> Result<std::net::SocketAddr, Self::Error> {
        match self {
            Endpoint::Http(http) => http.try_into(),
            _ => Err(IoError::new(
                ErrorKind::InvalidInput,
                "Only `Http` can `into` `SocketAddr`.",
            )),
        }
    }
}

fn uri_to_addr(uri: Uri) -> String {
    uri_parts_to_addr(&uri.into_parts())
}

fn uri_parts_to_addr(parts: &http::uri::Parts) -> String {
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
    addr
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
    /// - `http://example.com:50042`
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
                ErrorKind::InvalidInput,
                "Only IPC can `into` `Path`.",
            )),
        }
    }
}

pub fn from_uri(uri: Uri) -> Result<Endpoint, crate::StdError> {
    if let Some(scheme) = uri.scheme_str() {
        match scheme {
            "http" | "https" => Ok(Endpoint::Http(Http(uri))),
            "ipc" | "uds" => {
                let mut addr = uri_to_addr(uri);
                match addr.chars().next() {
                    Some('.') | Some('~') => { /* relative path */ }
                    Some('/') => { /* absolute path */ }
                    _ => addr = "/".to_owned() + &addr,
                }
                Ok(Endpoint::Ipc(Ipc(addr.parse()?)))
            }
            _ => Err(IoError::new(
                ErrorKind::InvalidInput,
                format!("Protocol {} not supported", scheme),
            )
            .into()),
        }
    } else {
        Err(IoError::new(ErrorKind::InvalidInput, "Protocol scheme is missed").into())
    }
}

impl TryInto<Uri> for Endpoint {
    type Error = http::Error;

    fn try_into(self) -> Result<Uri, Self::Error> {
        match self {
            Endpoint::Ipc(Ipc(pb)) => format!(
                "ipc:{}{}",
                if pb.is_absolute() { "/" } else { "//" },
                pb.to_string_lossy()
            )
            .parse()
            .map_err(|e: http::uri::InvalidUri| e.into()),

            Endpoint::Http(Http(uri)) => Ok(uri),
        }
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

    // test URIs & paths:
    const IPC_ABS: &str = "ipc://tmp/dir/file";
    const IPC_REL: [&str; 2] = ["ipc://./tmp/file", "ipc://../tmp/file"];
    const HTTP_URI: [&str; 3] = [
        "http://[::1]:50042/",
        "http://127.0.0.1:50042/",
        "http://sub.example.com:61191/",
    ];
    const HTTP_SOC: [&str; 2] = ["[::1]:50042", "127.0.0.1:50042"];

    #[test]
    fn from_str_absolete_ipc() {
        let endpoint: Result<Endpoint, _> = IPC_ABS.parse();
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
        for path in IPC_REL.iter() {
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
        for (i, uri) in HTTP_URI.iter().enumerate() {
            let endpoint: Result<Endpoint, _> = uri.parse();
            assert!(endpoint.is_ok());

            if let Ok(endpoint) = endpoint {
                match endpoint {
                    Endpoint::Http(inner) => {
                        // assert_eq!(HTTP_SOC[i], &inner.to_string());
                        assert_eq!(HTTP_URI[i], &inner.to_string());
                    }
                    Endpoint::Ipc(_) => panic!("expected HTTP"),
                }
            } else {
                unreachable!();
            }
        }
    }

    #[test]
    fn to_uri_absolete_ipc() {
        let endpoint: Endpoint = IPC_ABS.parse().unwrap();
        let result: Result<Uri, _> = endpoint.try_into();
        assert!(result.is_ok());
        let uri = result.unwrap();
        assert_eq!(IPC_ABS, uri.to_string())
    }

    #[test]
    fn to_uri_relative_ipc() {
        for uri in IPC_REL.iter() {
            let endpoint: Endpoint = uri.parse().unwrap();
            let result: Result<Uri, _> = endpoint.try_into();
            assert!(result.is_ok());
            assert_eq!(uri, &result.unwrap().to_string())
        }
    }

    #[test]
    fn to_uri_http() {
        for expected in HTTP_URI.iter() {
            let endpoint: Endpoint = expected.parse().unwrap();
            let result: Result<Uri, _> = endpoint.try_into();
            assert!(result.is_ok());
            let uri = result.unwrap().to_string();
            assert_eq!(expected, &uri);
        }
    }

    #[test]
    fn to_soc_http() {
        for (i, uri) in HTTP_URI[..1].iter().enumerate() {
            let endpoint: Endpoint = uri.parse().unwrap();
            let result: Result<std::net::SocketAddr, _> = endpoint.try_into();
            assert!(result.is_ok());
            let soc = result.unwrap().to_string();
            assert_eq!(HTTP_SOC[i], &soc);
        }
    }
}
