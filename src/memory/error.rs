use std::{string::FromUtf8Error, fmt::Display};

#[derive(Debug)]
pub enum ProcessError {
    ProcessNotFound,
    NotEnoughPermissions,
    IoError{
        inner: std::io::Error
    },
    FromUtf8Error,
    ConvertionError,

    SignatureNotFound(String),

    OsError{
        #[cfg(target_os = "linux")]
        inner: nix::errno::Errno,
        #[cfg(target_os = "windows")]
        inner: windows::core::Error,
    },
}

impl From<std::io::Error> for ProcessError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError{ inner: value }
    }
}

impl From<std::num::ParseIntError> for ProcessError {
    fn from(_: std::num::ParseIntError) -> Self {
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

// Windows only
#[cfg(target_os = "windows")]
impl From<windows::core::Error> for ProcessError {
    fn from(inner: windows::core::Error) -> Self {
        dbg!(&inner);
        Self::OsError{
            inner
        }// TODO add code value
    }
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessError::ProcessNotFound => 
                write!(f, "Process not found!"),
            ProcessError::IoError { .. } => 
                write!(f, "Got I/O error!"),
            ProcessError::FromUtf8Error => 
                write!(f, "Got Error when converting bytes to string!"),
            ProcessError::ConvertionError => 
                write!(f, "Got error during type convertion"),
            ProcessError::SignatureNotFound(v) => 
                write!(f, "Cannot found signature {}", v),
            ProcessError::OsError { .. } => 
                write!(f, "Got OS error"),
            ProcessError::NotEnoughPermissions => 
                write!(
                    f, 
                    "Not enough permissions to run, please run as sudo"
                ),
        }
    }
}

impl std::error::Error for ProcessError {}
