pub mod database;
pub mod error;
mod jsonrpc;

use bigdecimal::BigDecimal;

pub enum Method {
    Create,
    Update,
    Delete,
    Binary(BinaryOps),
}

pub enum BinaryOps {
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub enum Param {
    Name(Box<str>),
    Number(BigDecimal),
}

pub trait JsonInternal {
    fn parse_method(&self) -> Method;
    fn parse_params(&self) -> Vec<Param>;
}

pub mod prelude {
    pub mod v1 {
        pub use crate::database::*;
        pub use crate::error::*;
        pub use crate::jsonrpc::v1::*;
        pub use crate::{JsonInternal, Method, Param};

        use std::str::FromStr;

        use bigdecimal::BigDecimal;

        pub struct RequestBuilder {
            body: ReqBody,
        }

        impl RequestBuilder {
            pub fn from_json(json_string: &str) -> Result<Self, ClientError> {
                let body: ReqBody = serde_json::from_str(json_string)?;
                Ok(RequestBuilder { body })
            }

            pub fn new(method: Method, params: Vec<String>, id: usize) -> Self {
                RequestBuilder {
                    body: ReqBody {
                        jsonrpc: "1.0".to_string(),
                        method: method.into(),
                        params,
                        id,
                    },
                }
            }

            pub fn build(self) -> Result<Vec<u8>, serde_json::Error> {
                let mut payload = serde_json::to_vec(&self.body)?;
                let checksum = crc32fast::hash(&payload);
                payload.extend_from_slice(&checksum.to_le_bytes());

                Ok(payload)
            }
        }

        pub struct ResponseBuilder {
            body: RespBody,
        }

        impl ResponseBuilder {
            pub fn new(result: String, id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: Some(result),
                        error: None,
                        id,
                    },
                }
            }

            pub fn success(id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: Some("success".to_string()),
                        error: None,
                        id,
                    },
                }
            }

            pub fn error(msg: ErrorMsg, id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: None,
                        error: Some(msg.into_inner()),
                        id,
                    },
                }
            }

            pub fn build(self) -> Vec<u8> {
                if let Ok(mut payload) = serde_json::to_vec(&self.body) {
                    let checksum = crc32fast::hash(&payload);
                    payload.extend_from_slice(&checksum.to_le_bytes());

                    payload
                } else {
                    todo!()
                }
            }
        }

        impl JsonInternal for ReqBody {
            fn parse_method(&self) -> Method {
                self.method.clone().into()
            }

            fn parse_params(&self) -> Vec<Param> {
                let results: Vec<Param> = self
                    .params
                    .clone()
                    .into_iter()
                    .map(|param| {
                        if let Ok(number) = BigDecimal::from_str(&param) {
                            Param::Number(number)
                        } else {
                            Param::Name(param.into_boxed_str())
                        }
                    })
                    .collect();

                results
            }
        }
    }
}

impl From<Method> for String {
    fn from(value: Method) -> Self {
        let str_slice = match value {
            Method::Create => "create",
            Method::Update => "update",
            Method::Delete => "delete",
            Method::Binary(BinaryOps::Add) => "add",
            Method::Binary(BinaryOps::Subtract) => "subtract",
            Method::Binary(BinaryOps::Multiply) => "multiply",
            Method::Binary(BinaryOps::Divide) => "divide",
        };

        str_slice.to_string()
    }
}

impl From<String> for Method {
    fn from(value: String) -> Self {
        match value.as_str() {
            "create" => Method::Create,
            "update" => Method::Update,
            "delete" => Method::Delete,
            "add" => Method::Binary(BinaryOps::Add),
            "subtract" => Method::Binary(BinaryOps::Subtract),
            "multiply" => Method::Binary(BinaryOps::Multiply),
            "divide" => Method::Binary(BinaryOps::Divide),
            _ => unreachable!(),
        }
    }
}
