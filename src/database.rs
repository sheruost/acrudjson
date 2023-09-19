use crate::error::{ErrorMsg, ServerError};
use crate::prelude::v1::*;
use crate::{BinaryOps, Method, Param};

use std::path::{Path, PathBuf};

use sled::{Db, Tree};
use zerocopy::{AsBytes, ByteOrder, ByteSlice, LittleEndian};

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
                            let result = match op {
                                BinaryOps::Add => left_value + right_value,
                                BinaryOps::Subtract => left_value - right_value,
                                BinaryOps::Multiply => left_value * right_value,
                                BinaryOps::Divide => left_value / right_value,
                            };

                            return ResponseBuilder::new(result.to_string(), c_id).build();
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

    fn create(&self, key: &str, value: f64) -> Result<(), ServerError> {
        Ok(self.tree.compare_and_swap(
            key.as_bytes(),
            None as Option<&[u8]>,
            Some(&value.to_le_bytes()),
        )??)
    }

    fn fetch(&self, key: &str) -> Result<Option<f64>, ServerError> {
        if let Some(fetched) = self.tree.get(key.as_bytes())? {
            let number = LittleEndian::read_f64(&fetched);
            Ok(Some(number))
        } else {
            unimplemented!()
        }
    }

    fn update(&self, key: &str, new_value: f64) -> Result<Option<f64>, ServerError> {
        if let Some(prev_value) = self.tree.insert(key.as_bytes(), &new_value.to_le_bytes())? {
            let number = LittleEndian::read_f64(&prev_value);
            Ok(Some(number))
        } else {
            unimplemented!()
        }
    }

    fn delete(&self, key: &str) -> Result<Option<String>, ServerError> {
        if let Some(removed) = self.tree.remove(key.as_bytes())? {
            let param = std::str::from_utf8(&removed)?;
            Ok(Some(param.to_string()))
        } else {
            unimplemented!()
        }
    }
}
