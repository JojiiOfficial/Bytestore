use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Bitcode(bitcode::Error),
    Bincode(bincode::Error),
    OutOfBounds,
    InvalidHeader,
    InvalidShift,
    Initialization,
    UnexpectedValue,
    UnsupportedOperation,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(..), Self::Io(..)) => true,
            (Self::Bitcode(..), Self::Bitcode(..)) => true,
            (Self::Bincode(..), Self::Bincode(..)) => true,
            (Self::OutOfBounds, Self::OutOfBounds) => true,
            (Self::InvalidHeader, Self::InvalidHeader) => true,
            (Self::Initialization, Self::Initialization) => true,
            (Self::UnexpectedValue, Self::UnexpectedValue) => true,
            (Self::UnsupportedOperation, Self::UnsupportedOperation) => true,
            (_, _) => false,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<bincode::Error> for Error {
    fn from(value: bincode::Error) -> Self {
        Self::Bincode(value)
    }
}

impl From<bitcode::Error> for Error {
    fn from(value: bitcode::Error) -> Self {
        Self::Bitcode(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
