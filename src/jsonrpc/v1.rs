use serde::{Deserialize, Serialize};

/// The JSON Request object following JSON-RPC 1.0 specification.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReqBody {
    /// string of version of JSON-RPC protocol. MUST be exactly "1.0"
    pub jsonrpc: String,
    /// string containing the name of invoke method from public.
    pub method: String,
    /// an array of string as parameter values used by method during invocation.
    pub params: Vec<String>,
    /// an identifier established by client must contain a number preferably in ascending order
    /// sequence.
    pub id: usize,
}

/// The JSON Response object following JSON-RPC 1.0 specification.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RespBody {
    /// the member is required on `success`, MUST NOT exist on `error` invoking the method.
    pub result: Option<String>,
    /// the member is required when there's an `error` invoking the method, MUST NOT exist on `success`.
    pub error: Option<String>,
    /// an identifier corresponding to `id` member in same JSON Request object.
    pub id: usize,
}
