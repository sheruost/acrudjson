use crate::error::{ErrorMsg, ServerError};
use crate::prelude::v1::*;
use crate::{BinaryOps, Method, Param};

use std::path::{Path, PathBuf};
use std::str::{self, FromStr};

use astro_float::{BigFloat, RoundingMode};
use sled::{Db, Tree};
use zerocopy::{AsBytes, ByteSlice};

const BIG_FLOAT_PRECISION: usize = 1024;

pub struct ConnectionPool {
    prefix: PathBuf,
    db: Db,
}

impl ConnectionPool {
    pub fn get_filepath(&self) -> &Path {
        self.prefix.as_path()
    }

    pub fn init(path: impl AsRef<Path>) -> Result<Self, ServerError> {
        let prefix = path.as_ref().to_path_buf();
        let db = sled::open(path)?;

        Ok(ConnectionPool { prefix, db })
    }

    pub fn open_user_database(&self, token: impl ByteSlice) -> Result<UserDatabase, ServerError> {
        let tree = self.db.open_tree(token.as_bytes())?;

        Ok(UserDatabase {
            token: token.to_vec(),
            tree,
        })
    }
}

pub struct UserDatabase {
    token: Vec<u8>,
    tree: Tree,
}

impl UserDatabase {
    pub fn get_token(&self) -> &[u8] {
        &self.token
    }

    pub fn transaction(&self, method: Method, params: Vec<Param>, c_id: usize) -> Vec<u8> {
        // resolve values from Params
        let mut param_iter = params.into_iter();
        let key = match param_iter.next() {
            Some(Param::Name(literal)) => literal,
            Some(_) => {
                return ResponseBuilder::error(
                    ErrorMsg::new(format!("the first parameter must be string literal.")),
                    c_id,
                )
                .build();
            }
            None => {
                return ResponseBuilder::error(ServerError::MissingParam(1).into(), c_id).build();
            }
        };

        match method {
            Method::Create => match param_iter.next() {
                Some(Param::Number(value)) => match self.create(&key, value) {
                    Ok(_) => {
                        return ResponseBuilder::success(c_id).build();
                    }
                    Err(e) => {
                        return ResponseBuilder::error(e.into(), c_id).build();
                    }
                },
                Some(_) => {
                    return ResponseBuilder::error(ServerError::ParseParamNumeric.into(), c_id)
                        .build();
                }
                None => {
                    return ResponseBuilder::error(ServerError::MissingParam(1).into(), c_id)
                        .build();
                }
            },
            Method::Update => match param_iter.next() {
                Some(Param::Number(new_value)) => match self.update(&key, new_value) {
                    Ok(_) => {
                        return ResponseBuilder::success(c_id).build();
                    }
                    Err(e) => {
                        return ResponseBuilder::error(e.into(), c_id).build();
                    }
                },
                Some(_) => {
                    return ResponseBuilder::error(
                        ErrorMsg::new(format!("the second parameter must be a number.")),
                        c_id,
                    )
                    .build();
                }
                None => {
                    return ResponseBuilder::error(ServerError::MissingParam(1).into(), c_id)
                        .build();
                }
            },
            Method::Delete => match self.delete(&key) {
                Ok(_) => {
                    return ResponseBuilder::success(c_id).build();
                }
                Err(e) => {
                    return ResponseBuilder::error(e.into(), c_id).build();
                }
            },
            Method::Binary(op) => match param_iter.next() {
                Some(Param::Name(second_key)) => match self.fetch(&key) {
                    Ok(Some(left_value)) => match self.fetch(&second_key) {
                        Ok(Some(right_value)) => {
                            match op {
                                BinaryOps::Add => left_value.add(
                                    &right_value,
                                    BIG_FLOAT_PRECISION,
                                    RoundingMode::ToEven,
                                ),
                                BinaryOps::Subtract => left_value.sub(
                                    &right_value,
                                    BIG_FLOAT_PRECISION,
                                    RoundingMode::ToEven,
                                ),
                                BinaryOps::Multiply => left_value.mul(
                                    &right_value,
                                    BIG_FLOAT_PRECISION,
                                    RoundingMode::ToEven,
                                ),
                                BinaryOps::Divide => left_value.div(
                                    &right_value,
                                    BIG_FLOAT_PRECISION,
                                    RoundingMode::ToEven,
                                ),
                            };

                            let result_string = serde_json::to_string(&left_value).unwrap();
                            return ResponseBuilder::new(result_string, c_id).build();
                        }
                        Ok(None) => {
                            return ResponseBuilder::error(
                                ServerError::DbKeyNotFound(second_key.to_string().into_boxed_str())
                                    .into(),
                                c_id,
                            )
                            .build();
                        }
                        Err(e) => {
                            return ResponseBuilder::error(e.into(), c_id).build();
                        }
                    },
                    Ok(None) => {
                        return ResponseBuilder::error(
                            ServerError::DbKeyNotFound(second_key.to_string().into_boxed_str())
                                .into(),
                            c_id,
                        )
                        .build();
                    }
                    Err(e) => {
                        return ResponseBuilder::error(e.into(), c_id).build();
                    }
                },
                Some(_) => {
                    return ResponseBuilder::error(
                        ErrorMsg::new(format!("the first parameter must be string literal.")),
                        c_id,
                    )
                    .build();
                }
                None => {
                    return ResponseBuilder::error(ServerError::MissingParam(1).into(), c_id)
                        .build();
                }
            },
        }
    }

    fn create(&self, key: &str, value: BigFloat) -> Result<(), ServerError> {
        let float_string = serde_json::to_string(&value).unwrap();
        Ok(self.tree.compare_and_swap(
            key.as_bytes(),
            None as Option<&[u8]>,
            Some(float_string.as_bytes()),
        )??)
    }

    fn fetch(&self, key: &str) -> Result<Option<BigFloat>, ServerError> {
        if let Some(fetched) = self.tree.get(key.as_bytes())? {
            let float_string = str::from_utf8(&fetched)?;
            let big_float = BigFloat::from_str(float_string).unwrap();
            Ok(Some(big_float))
        } else {
            Ok(None)
        }
    }

    fn update(&self, key: &str, new_value: BigFloat) -> Result<Option<BigFloat>, ServerError> {
        let float_string = serde_json::to_string(&new_value).unwrap();
        if let Some(prev_value) = self.tree.insert(key.as_bytes(), float_string.as_bytes())? {
            let float_string = str::from_utf8(&prev_value)?;
            let big_float = BigFloat::from_str(float_string).unwrap();
            Ok(Some(big_float))
        } else {
            Ok(None)
        }
    }

    fn delete(&self, key: &str) -> Result<Option<String>, ServerError> {
        if let Some(removed) = self.tree.remove(key.as_bytes())? {
            let param = str::from_utf8(&removed)?;
            Ok(Some(param.to_string()))
        } else {
            Ok(None)
        }
    }
}
