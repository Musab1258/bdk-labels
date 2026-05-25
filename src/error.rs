use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid BIP329 structure: {0}")]
    Validation(String),
    #[error("Custom Database error: {0}")]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_conversion() {
        fn trigger_conversion() -> Result<(), Error> {
            let io_error = io::Error::new(io::ErrorKind::Other, "Forced IO Failure");
            Err(io_error)?
        }

        let result = trigger_conversion();

        assert!(matches!(result, Err(Error::Io(_))));
    }

    #[test]
    fn test_serde_json_error_conversion() {
        fn trigger_conversion() -> Result<(), Error> {
            let json_error =
                serde_json::from_str::<serde_json::Value>("{ corrupted json string}").unwrap_err();
            Err(json_error)?
        }

        let result = trigger_conversion();

        assert!(matches!(result, Err(Error::Json(_))));
    }
}
