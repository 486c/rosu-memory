use std::{num::ParseIntError, str::Utf8Error, string::FromUtf8Error};

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ProcessError {
    #[error("Process not found!")]
    ProcessNotFound,
    #[error("Executable path not found!")]
    ExecutablePathNotFound,
    #[error("Not enough permissions to run, please run as sudo")]
    NotEnoughPermissions,
    #[error("Got I/O error!")]
    IoError {
        #[from]
        inner: std::io::Error,
    },
    #[error("Got error when converting bytes to string!")]
    FromUtf8Error,
    #[error("Got error during type conversion")]
    ConvertionError,
    #[error("Trying to read bad address\nAddress: {0:X}, Length: {1:X}")]
    BadAddress(usize, usize),
    #[error("Cannot find signature {0}")]
    SignatureNotFound(String),
    #[error("Got OS error")]
    OsError {
        #[cfg(target_os = "linux")]
        #[source]
        inner: nix::errno::Errno,
        #[cfg(target_os = "windows")]
        #[from]
        inner: windows::core::Error, // TODO: add code value
    },
}

impl From<ParseIntError> for ProcessError {
    fn from(_: ParseIntError) -> Self {
        Self::ConvertionError
    }
}

impl From<std::num::TryFromIntError> for ProcessError {
    fn from(_: std::num::TryFromIntError) -> Self {
        Self::ConvertionError
    }
}

impl From<FromUtf8Error> for ProcessError {
    fn from(_: FromUtf8Error) -> Self {
        Self::FromUtf8Error
    }
}

impl From<Utf8Error> for ProcessError {
    fn from(_: Utf8Error) -> Self {
        Self::FromUtf8Error
    }
}

// Linux only
#[cfg(target_os = "linux")]
impl From<nix::errno::Errno> for ProcessError {
    fn from(inner: nix::errno::Errno) -> Self {
        match inner {
            nix::errno::Errno::EPERM =>
                Self::NotEnoughPermissions,
            nix::errno::Errno::ESRCH =>
                Self::ProcessNotFound,
            _ => Self::OsError { inner },
        }
    }
}

#[derive(Debug, ThisError)]
pub enum ParseSignatureError {
    #[error("Invalid string length {0}")]
    InvalidLength(usize),
    #[error("Failed to parse integer")]
    InvalidInt {
        #[from]
        inner: ParseIntError,
    },
}
