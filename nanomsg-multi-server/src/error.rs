use nanomsg::result::Error as NanoError;

#[derive(Debug, PartialEq)]
pub enum ServerError {
    SocketError(NanoError),
}

impl From<NanoError> for ServerError {
    fn from(source: NanoError) -> ServerError {
        ServerError::SocketError(source)
    }
}
