//! A CRUD API for arithmetic operations with astronomically large floating point numbers based on [JSON-RPC Specification].
//!
//! [JSON-RPC Specification]: https://www.jsonrpc.org/specification

/// connection pool and data transaction queries for user database instances.
pub mod database;
/// server and client error types with error message constructor for JSON response payload.
pub mod error;
mod jsonrpc;

use std::fmt;

use bigdecimal::BigDecimal;

/// A JSON object to invoke basic CRUD implementation of `acrudjson` through
/// JSON-RPC protocol. It can be used for [`RequestBuilder`] in frontend without
/// parsing JSON string.
///
/// NOTE:
///     - method name beginning with `rpc` followed by `,` are preserved for RPC internal
///     methods based on [JSON-RPC 2.0 Specification].
///
/// [`RequestBuilder`]: crate::prelude::v1::RequestBuilder
/// [JSON-RPC 2.0 Specification]: https://www.jsonrpc.org/specification
#[derive(Debug)]
pub enum Method {
    Create,
    Read,
    Update,
    Delete,
    Binary(BinaryOps),
}

/// Provide arithmetic of binary numbers wrapped by [`Method`].
///
/// [`Method`]: crate::Method
#[derive(Debug)]
pub enum BinaryOps {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// A JSON object to provide key or value members for variable assignment or storing entries
/// in [`UserDatabase`] following with [`Method`].
///
/// NOTE:
///     - `Param::Number(number)` is provided by [bigdecimal] for decimal representation and large
///     floating number computation.
///
/// [`UserDatabase`]: crate::database::UserDatabase
/// [`Method`]: crate::Method
/// [bigdecimal]: https://docs.rs/bigdecimal/latest/bigdecimal/
#[derive(Debug)]
pub enum Param {
    /// contains immutable utf8-string parsed from JSON string object
    Name(Box<str>),
    /// contains big decimal number parsed from JSON string object.
    Number(BigDecimal),
}

pub trait JsonInternal {
    /// parse JSON member "method" value into `Method`.
    fn parse_method(&self) -> Method;
    /// parse JSON member "params" array values into `Vec<Param>`.
    fn parse_params(&self) -> Vec<Param>;
}

/// import `struct` implementations following JSON-RPC specification.
pub mod prelude {
    /// JSON-RPC 1.0 specification
    pub mod v1 {
        pub use crate::database::*;
        pub use crate::error::*;
        pub use crate::jsonrpc::v1::*;
        pub use crate::{JsonInternal, Method, Param};

        use std::str::FromStr;

        use bigdecimal::BigDecimal;

        /// Used to compose JSON request based on JSON-RPC 1.0 specification.
        ///
        /// NOTE:
        ///     - the result payload contains a `u32` crc32 checksum in little-endianness in tail
        ///     bytes.
        pub struct RequestBuilder {
            body: ReqBody,
        }

        impl RequestBuilder {
            /// parse JSON string into [`ReqBody`] to initialise the builder.
            ///
            /// [`ReqBody`]: crate::prelude::v1::ReqBody
            pub fn from_json(json_string: &str) -> Result<Self, ClientError> {
                let body: ReqBody = serde_json::from_str(json_string)?;
                Ok(RequestBuilder { body })
            }

            /// creates and return [`ReqBody`] as builder.
            ///
            /// [`ReqBody`]: crate::prelude::v1::ReqBody
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

            /// calculate crc32 checksum then append the bytes after request body.
            pub fn build(self) -> Result<Vec<u8>, serde_json::Error> {
                let mut payload = serde_json::to_vec(&self.body)?;
                let checksum = crc32fast::hash(&payload);
                payload.extend_from_slice(&checksum.to_le_bytes());

                Ok(payload)
            }
        }

        /// Used to compose JSON response based on JSON-RPC 1.0 specification.
        ///
        /// NOTE:
        ///     - the result payload contains a `u32` crc32 checksum in little-endianness in tail
        ///     bytes.
        pub struct ResponseBuilder {
            body: RespBody,
        }

        impl ResponseBuilder {
            /// parse JSON string into [`RespBody`] to initialise the builder.
            ///
            /// [`RespBody`]: crate::prelude::v1::RespBody
            pub fn from_json(json_string: &str) -> Result<Self, ClientError> {
                let body: RespBody = serde_json::from_str(json_string)?;
                Ok(ResponseBuilder { body })
            }

            /// creates and return [`RespBody`] as builder.
            ///
            /// [`RespBody`]: crate::prelude::v1::RespBody
            pub fn new(result: String, id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: Some(result),
                        error: None,
                        id,
                    },
                }
            }

            /// compose JSON response when target request proceeds successfully.
            /// NOTE: the `id` should be same as target JSON request.
            pub fn success(id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: Some("success".to_string()),
                        error: None,
                        id,
                    },
                }
            }

            /// compose JSON response when target request proceeds failed with `ErrorMsg`
            /// indicates error message in JSON "error" field.
            /// NOTE: the `id` should be same as target JSON request.
            pub fn error(msg: ErrorMsg, id: usize) -> Self {
                ResponseBuilder {
                    body: RespBody {
                        result: None,
                        error: Some(msg.into_inner()),
                        id,
                    },
                }
            }

            /// calculate crc32 checksum then append the bytes after response body.
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
            Method::Read => "read",
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
            "read" => Method::Read,
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

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Method::Create => write!(f, "create"),
            Method::Read => write!(f, "read"),
            Method::Update => write!(f, "update"),
            Method::Delete => write!(f, "delete"),
            Method::Binary(BinaryOps::Add) => write!(f, "add"),
            Method::Binary(BinaryOps::Subtract) => write!(f, "subtract"),
            Method::Binary(BinaryOps::Multiply) => write!(f, "multiply"),
            Method::Binary(BinaryOps::Divide) => write!(f, "divide"),
        }
    }
}

impl fmt::Display for BinaryOps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BinaryOps::Add => write!(f, "add"),
            BinaryOps::Subtract => write!(f, "subtract"),
            BinaryOps::Multiply => write!(f, "multiply"),
            BinaryOps::Divide => write!(f, "divide"),
        }
    }
}
