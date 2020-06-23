use std::error::Error;

pub type ClientResult<T> = std::result::Result<T, ClientError>;

#[derive(Debug)]
pub enum ClientError {
    ParcelNotFound,
    Unauthorized,
    ServerError,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            ClientError::ParcelNotFound => f.write_str("Parcel number could not be found"),
            ClientError::Unauthorized => f.write_str("Unauthorized, check your API key"),
            ClientError::ServerError => f.write_str("Could not reach server or parse response"),
        }
    }
}

impl Error for ClientError {}

impl From<surf::Exception> for ClientError {
    fn from(_err: surf::Exception) -> Self {
        ClientError::ServerError
    }
}

impl From<std::io::Error> for ClientError {
    fn from(_err: std::io::Error) -> Self {
        ClientError::ServerError
    }
}
