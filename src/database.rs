use crate::{error::ServerError, BinaryOps, Method, Param};

use std::path::{Path, PathBuf};
use std::str::{self, FromStr};

use bigdecimal::BigDecimal;
use log::{error, info};
use sled::{Db, Tree};
use zerocopy::{AsBytes, ByteSlice};

/// The connection pool to maintain [`sled`] database running instance and path prefix to storage
/// file.
///
/// [`sled`]: https://docs.rs/sled/latest/sled/
pub struct ConnectionPool {
    prefix: PathBuf,
    db: Db,
}

impl ConnectionPool {
    /// get the filepath of `sled::Db` storage file.
    pub fn get_filepath(&self) -> &Path {
        self.prefix.as_path()
    }

    /// initialise and start the connection pool by provided filepath as file prefix of `sled`
    /// database instance.
    pub fn init(path: impl AsRef<Path>) -> Result<Self, ServerError> {
        let prefix = path.as_ref().to_path_buf();
        let db = sled::open(path)?;

        Ok(ConnectionPool { prefix, db })
    }

    /// open user storage tree by provided `user token`.
    pub fn open_user_database(&self, token: impl ByteSlice) -> Result<UserDatabase, ServerError> {
        let tree = self.db.open_tree(token.as_bytes())?;

        Ok(UserDatabase {
            token: token.to_vec(),
            tree,
        })
    }
}

/// A user database tree identified by `token`(TODO) implemented with `sled`, which is a high-performance, thread-safe
/// and fully atomic embedded database.
///
/// TODO: implement Token instance which can be used to generate user-token for individial access
/// to data storage in frontend.
pub struct UserDatabase {
    token: Vec<u8>,
    tree: Tree,
}

impl UserDatabase {
    /// get user `token`
    pub fn get_token(&self) -> &[u8] {
        &self.token
    }

    /// perform ACID transactions by provided [`Method`] and [`Param`]s from JSON Request body,
    /// and return the result of invocation for JSON Response "result" and "error" object members.
    ///
    /// [`Method`]: crate::Method
    /// [`Param`]: crate::Param
    pub fn transaction(
        &self,
        method: Method,
        params: Vec<Param>,
    ) -> Result<Option<String>, ServerError> {
        // resolve values from Params
        let mut param_iter = params.into_iter();
        let key = match param_iter.next() {
            Some(Param::Name(literal)) => literal,
            Some(_) => {
                let e = ServerError::MissingName(0);
                error!("{e}");
                return Err(e);
            }
            None => {
                let e = ServerError::MissingParam(1);
                error!("{e}");
                return Err(e);
            }
        };

        let result = match method {
            Method::Create => match param_iter.next() {
                Some(Param::Number(value)) => match self.create(&key, value) {
                    Ok(_) => Ok(None),
                    Err(e) => {
                        error!("{e}");
                        Err(e)
                    }
                },
                Some(_) => Err(ServerError::MissingNumber(1)),
                None => Err(ServerError::MissingParam(1)),
            },
            Method::Read => match self.fetch(&key) {
                Ok(decimal) => Ok(Some(decimal.to_string())),
                Err(e) => {
                    error!("{e}");
                    Err(e)
                }
            },
            Method::Update => match param_iter.next() {
                Some(Param::Number(new_value)) => match self.update(&key, new_value) {
                    Ok(_) => Ok(None),
                    Err(e) => {
                        error!("{e}");
                        Err(e)
                    }
                },
                Some(_) => Err(ServerError::MissingNumber(1)),
                None => Err(ServerError::MissingParam(1)),
            },
            Method::Delete => match self.delete(&key) {
                Ok(_) => Ok(None),
                Err(e) => {
                    // print error message of custom DbKeyNotFound error.
                    error!("{e}");
                    Err(e)
                }
            },
            Method::Binary(op) => match self.fetch(&key) {
                Ok(left_value) => match param_iter.next() {
                    Some(Param::Name(second_key)) => match self.fetch(&second_key) {
                        Ok(right_value) => {
                            let res = match op {
                                BinaryOps::Add => left_value + right_value,
                                BinaryOps::Subtract => left_value - right_value,
                                BinaryOps::Multiply => left_value * right_value,
                                BinaryOps::Divide => left_value / right_value,
                            };

                            Ok(Some(res.to_string()))
                        }
                        Err(e) => {
                            error!("{e}");
                            Err(e)
                        }
                    },
                    Some(Param::Number(right_value)) => {
                        info!(
                            "performing binary operation, method = {}, LHS = {}, RHS = {}",
                            op,
                            left_value.to_string(),
                            right_value.to_string()
                        );
                        let res = match op {
                            BinaryOps::Add => left_value + right_value,
                            BinaryOps::Subtract => left_value - right_value,
                            BinaryOps::Multiply => left_value * right_value,
                            BinaryOps::Divide => left_value / right_value,
                        };
                        Ok(Some(res.to_string()))
                    }
                    None => Err(ServerError::MissingParam(1)),
                },
                Err(e) => Err(e),
            },
        };

        result
    }

    fn create(&self, key: &str, value: BigDecimal) -> Result<(), ServerError> {
        let float_string = value.to_string();
        match self.tree.compare_and_swap(
            key.as_bytes(),
            None as Option<&[u8]>,
            Some(float_string.as_bytes()),
        )? {
            Ok(_) => {
                info!("create new key entry [\"{key}\"] with number = {float_string}");
                Ok(())
            }
            Err(cas) => Err(cas.into()),
        }
    }

    fn fetch(&self, key: &str) -> Result<BigDecimal, ServerError> {
        if let Some(fetched) = self.tree.get(key.as_bytes())? {
            let float_string = str::from_utf8(fetched.as_bytes())?;
            let big_float = BigDecimal::from_str(float_string)?;
            info!("fetch [\"{key}\"] value: {float_string}");
            Ok(big_float)
        } else {
            Err(ServerError::DbKeyNotFound(key.into()))
        }
    }

    fn update(&self, key: &str, new_value: BigDecimal) -> Result<(), ServerError> {
        if self.tree.contains_key(key.as_bytes())? {
            let new_float_string = new_value.to_string();
            let old_val_bytes = self
                .tree
                .insert(key.as_bytes(), new_float_string.as_bytes())?
                .ok_or(ServerError::DbEmptyValue(key.into()))?;
            let old_float_string = str::from_utf8(old_val_bytes.as_bytes())?;
            info!("update [\"{key}\"] value from {old_float_string} to {new_float_string}");
            Ok(())
        } else {
            Err(ServerError::DbKeyUpdate(key.into()))
        }
    }

    fn delete(&self, key: &str) -> Result<(), ServerError> {
        if let Some(_deleted) = self.tree.remove(key.as_bytes())? {
            info!("[\"{key}\"] entry has been deleted from user database.");
            Ok(())
        } else {
            Err(ServerError::DbKeyNotFound(key.into()))
        }
    }
}
