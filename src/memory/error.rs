use std::{
    string::FromUtf8Error, 
    fmt::Display, 
    str::Utf8Error,
    num::ParseIntError,
    error::Error
};


#[derive(Debug)]
pub enum ProcessError {
    ProcessNotFound,
    NotEnoughPermissions,
    IoError{
        inner: std::io::Error
    },
    FromUtf8Error,
    ConvertionError,
    BadAddress(usize, usize),
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

// Windows only
#[cfg(target_os = "windows")]
impl From<windows::core::Error> for ProcessError {
    fn from(inner: windows::core::Error) -> Self {
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
            ProcessError::BadAddress(addr, len) => {
                let _  = writeln!(f, "Trying to read bad address");
                writeln!(f, "Address: {:X}, Length: {:X}", addr, len)
            },
        }
    }
}

impl std::error::Error for ProcessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ProcessError::ProcessNotFound => None,
            ProcessError::NotEnoughPermissions => None,
            ProcessError::IoError { inner } => Some(inner),
            ProcessError::FromUtf8Error => None,
            ProcessError::ConvertionError => None,
            ProcessError::SignatureNotFound(_) => None,
            ProcessError::OsError { inner } => Some(inner),
            ProcessError::BadAddress(..) => None,
        }
    }
}

#[derive(Debug)]
pub enum ParseSignatureError {
    InvalidLength(usize),
    InvalidInt { inner: ParseIntError },
}

impl From<ParseIntError> for ParseSignatureError {
    fn from(inner: ParseIntError) -> Self {
        Self::InvalidInt { inner }
    }
}

impl Error for ParseSignatureError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseSignatureError::InvalidLength(_) => None,
            ParseSignatureError::InvalidInt { inner } => Some(inner),
        }
    }
}

impl Display for ParseSignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseSignatureError::InvalidLength(len) => write!(f, "Invalid string length {len}"),
            ParseSignatureError::InvalidInt { .. } => f.write_str("Failed to parse integer"),
        }
    }
}
