#[derive(Debug)]
pub enum ProcessError {
    ProcessNotFound,
    IoError{
        inner: std::io::Error
    },
    ConvertionError
}

impl From<std::io::Error> for ProcessError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError{ inner: value }
    }
}

impl From<std::num::ParseIntError> for ProcessError {
    fn from(value: std::num::ParseIntError) -> Self {
        Self::ConvertionError
    }
}
