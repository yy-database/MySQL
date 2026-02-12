use std::fmt;
use yykv_types::DsError;

#[derive(Debug)]
pub enum S3Error {
    InvalidAccessKeyId,
    SignatureDoesNotMatch,
    NoSuchBucket,
    NoSuchKey,
    Internal(String),
    Io(std::io::Error),
    Xml(String),
    Protocol(DsError),
}

impl fmt::Display for S3Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Error::InvalidAccessKeyId => write!(f, "Invalid Access Key"),
            S3Error::SignatureDoesNotMatch => write!(f, "Signature Does Not Match"),
            S3Error::NoSuchBucket => write!(f, "Bucket Not Found"),
            S3Error::NoSuchKey => write!(f, "Object Not Found"),
            S3Error::Internal(s) => write!(f, "Internal Error: {}", s),
            S3Error::Io(e) => write!(f, "IO Error: {}", e),
            S3Error::Xml(s) => write!(f, "XML Error: {}", s),
            S3Error::Protocol(e) => write!(f, "Protocol Error: {}", e),
        }
    }
}

impl std::error::Error for S3Error {}

impl From<std::io::Error> for S3Error {
    fn from(e: std::io::Error) -> Self {
        S3Error::Io(e)
    }
}

impl From<DsError> for S3Error {
    fn from(e: DsError) -> Self {
        S3Error::Protocol(e)
    }
}

pub type Result<T> = std::result::Result<T, S3Error>;
