// Error structures

use tokio_rustls::rustls::Error as TlsError;

#[derive(Debug)]
pub enum Error {
    ConnectionError(String),
    InvalidArgument(String),
    InvalidCertificate(String),
    IOError(std::io::Error),
    TLSError(TlsError),
    ProtobufError(protobuf::Error),
    DatabaseError(String),
    InvalidInput(String),
    ConfigError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IOError(err)
    }
}

impl From<TlsError> for Error {
    fn from(err: TlsError) -> Self {
        Error::TLSError(err)
    }
}

impl From<protobuf::Error> for Error {
    fn from(err: protobuf::Error) -> Self {
        Error::ProtobufError(err)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::DatabaseError(format!("Database error: {}", err))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidCertificate(msg) => write!(f, "Invalid certificate provided: {}", msg),
            Error::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Error::IOError(err) => write!(f, "I/O error: {}", err),
            Error::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Error::TLSError(err) => write!(f, "TLS error: {}", err),
            Error::ProtobufError(err) => write!(f, "Protobuf error: {}", err),
            Error::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Error::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}
