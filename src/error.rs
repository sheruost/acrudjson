use thiserror::Error;

//TODO: organise errors into Json format with predefined error code.
//e.g.
//#[error("JSON RPC error_code: {code}, error_message: {msg}")]
//JsonRpc { code: ErrorCode, msg: Box<str> },
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("client didn't receive JSON response due to timeout.")]
    Timeout,
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("JSON RPC checksum unmatched, expect: {expect}, actual: {actual}")]
    ChecksumUnmatch { expect: u32, actual: u32 },
    #[error("JSON parsing error: {0}")]
    ParseJson(#[from] serde_json::Error),
    #[error("failed to parse param values into utf8-string, error: {0}")]
    ParseParamLiteral(#[from] std::str::Utf8Error),
    #[error("failed to parse param values into big decimal number.")]
    ParseParamNumeric(#[from] bigdecimal::ParseBigDecimalError),
    #[error("missing {0} parameter")]
    MissingParam(usize),
    #[error("the parameter at index {0} must be a name.")]
    MissingName(usize),
    #[error("the parameter at index {0} must be decimal number.")]
    MissingNumber(usize),
    #[error("`{0}` is not found in user database")]
    DbKeyNotFound(Box<str>),
    #[error("key [\"{0}\"] does not hold any value.")]
    DbEmptyValue(Box<str>),
    #[error("`{0}` does not exist so it cannot be updated from user database")]
    DbKeyUpdate(Box<str>),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("failed to create new key-value pair, the key entry is already existed.")]
    SledCas(#[from] sled::CompareAndSwapError),
    #[error("ACID transaction error from user database, reason: {0}")]
    SledInternal(#[from] sled::Error),
    #[error("value error, expect: {expect}, actual: {actual}")]
    ValueError { expect: Box<str>, actual: Box<str> },
}

/// The content of error message required by JSON "error" attribute in JSON-RPC response.
pub struct ErrorMsg(String);

impl ErrorMsg {
    /// create new `ErrorMsg`
    pub fn new(msg: String) -> Self {
        ErrorMsg(msg)
    }

    /// consume `Self` and return inner value.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<ServerError> for ErrorMsg {
    fn from(value: ServerError) -> Self {
        match value {
            ServerError::ChecksumUnmatch { expect, actual } => ErrorMsg(format!(
                "JSON RPC checksum unmatched, expect: {expect}, actual: {actual}"
            )),
            ServerError::ParseJson(_) => ErrorMsg("failed to parse JSON attributes.".to_string()),
            ServerError::ParseParamLiteral(_) => {
                ErrorMsg("failed to parse parameter into utf8-string.".to_string())
            }
            ServerError::ParseParamNumeric(_) => {
                ErrorMsg("failed to parse paramater into floating number.".to_string())
            }
            ServerError::MissingParam(count) => ErrorMsg(format!("missing {count} parameter.")),
            ServerError::MissingName(idx) => ErrorMsg(format!("index {idx} must be a name.")),
            ServerError::MissingNumber(idx) => {
                ErrorMsg(format!("index {idx} must be decimal number."))
            }
            ServerError::DbKeyNotFound(key) => ErrorMsg(format!("[\"{key}\"] not found.")),
            ServerError::DbEmptyValue(key) => ErrorMsg(format!("[\"{key}\"] has empty value.")),
            ServerError::DbKeyUpdate(key) => ErrorMsg(format!("[\"{key}\"] does not exist.")),
            ServerError::SledCas(_) => {
                ErrorMsg("failed to create new value in user database".to_string())
            }
            ServerError::SledInternal(_) => {
                ErrorMsg("failed to fetch or update value in user database.".to_string())
            }
            ServerError::ValueError { .. } => {
                ErrorMsg("failed to parse decimal number by requesting name.".to_string())
            }
            _ => ErrorMsg("internal I/O error.".to_string()),
        }
    }
}
