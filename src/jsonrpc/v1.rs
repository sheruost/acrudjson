use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReqBody {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<String>,
    pub id: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct RespBody {
    pub result: Option<String>,
    pub error: Option<String>,
    pub id: usize,
}
