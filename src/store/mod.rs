//! [`Store`]: a [`Backend`] on redb.

mod codec;

use crate::number::check;
use crate::{Backend, Error, Op, Operand};

use std::path::Path;

use bigdecimal::BigDecimal;
use codec::{decode, encode};
use redb::backends::InMemoryBackend;
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};

/// One table for all namespaces, keyed by `(namespace, key)`.
const TABLE: TableDefinition<(&str, &str), &[u8]> = TableDefinition::new("acrudjson.v1");

/// Decimal numbers stored in redb, grouped by namespace.
///
/// A namespace is any string, such as a user id. Share a `Store` with `Arc`.
#[derive(Debug)]
pub struct Store {
    db: Database,
}

impl Store {
    /// Opens or creates the database file at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Self::init(Database::create(path)?)
    }

    /// A store in memory only, for tests.
    pub fn in_memory() -> Result<Self, Error> {
        Self::init(Database::builder().create_with_backend(InMemoryBackend::new())?)
    }

    fn init(db: Database) -> Result<Self, Error> {
        // Create the table so reads never find it missing.
        let txn = db.begin_write()?;
        txn.open_table(TABLE)?;
        txn.commit()?;
        Ok(Store { db })
    }
}

impl Backend for Store {
    fn create(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<(), Error> {
        check(value)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(TABLE)?;
            if table.get((ns, key))?.is_some() {
                return Err(Error::KeyExists);
            }
            table.insert((ns, key), encode(value).as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    fn read(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(TABLE)?;
        fetch(&table, ns, key)
    }

    fn update(&self, ns: &str, key: &str, value: &BigDecimal) -> Result<BigDecimal, Error> {
        check(value)?;
        let txn = self.db.begin_write()?;
        let previous = {
            let mut table = txn.open_table(TABLE)?;
            let previous = table.insert((ns, key), encode(value).as_slice())?;
            match previous {
                Some(guard) => decode(guard.value())?,
                // Not committed, so the insert is rolled back.
                None => return Err(Error::KeyNotFound),
            }
        };
        txn.commit()?;
        Ok(previous)
    }

    fn delete(&self, ns: &str, key: &str) -> Result<BigDecimal, Error> {
        let txn = self.db.begin_write()?;
        let removed = {
            let mut table = txn.open_table(TABLE)?;
            let removed = table.remove((ns, key))?;
            match removed {
                Some(guard) => decode(guard.value())?,
                None => return Err(Error::KeyNotFound),
            }
        };
        txn.commit()?;
        Ok(removed)
    }

    fn compute<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        _op: O,
        lhs: &L,
        rhs: &R,
    ) -> Result<BigDecimal, Error> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(TABLE)?;
        let l = lhs.resolve(|key| fetch(&table, ns, key))?;
        let r = rhs.resolve(|key| fetch(&table, ns, key))?;
        O::apply(&l, &r)
    }

    fn compute_into<O: Op, L: Operand + ?Sized, R: Operand + ?Sized>(
        &self,
        ns: &str,
        _op: O,
        lhs: &L,
        rhs: &R,
        into: &str,
    ) -> Result<BigDecimal, Error> {
        let txn = self.db.begin_write()?;
        let result = {
            let mut table = txn.open_table(TABLE)?;
            let result = {
                let l = lhs.resolve(|key| fetch(&table, ns, key))?;
                let r = rhs.resolve(|key| fetch(&table, ns, key))?;
                O::apply(&l, &r)?
            };
            table.insert((ns, into), encode(&result).as_slice())?;
            result
        };
        txn.commit()?;
        Ok(result)
    }
}

type Key = (&'static str, &'static str);

fn fetch<T>(table: &T, ns: &str, key: &str) -> Result<BigDecimal, Error>
where
    T: ReadableTable<Key, &'static [u8]>,
{
    match table.get((ns, key))? {
        Some(guard) => decode(guard.value()),
        None => Err(Error::KeyNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> BigDecimal {
        s.parse().unwrap()
    }

    #[test]
    fn crud() {
        let store = Store::in_memory().unwrap();
        store.create("a", "x", &dec("1.5")).unwrap();
        assert!(matches!(
            store.create("a", "x", &dec("2")),
            Err(Error::KeyExists)
        ));
        assert!(
            matches!(store.read("b", "x"), Err(Error::KeyNotFound)),
            "namespaces are isolated"
        );
        assert_eq!(store.update("a", "x", &dec("2")).unwrap(), dec("1.5"));
        assert!(matches!(
            store.update("a", "y", &dec("2")),
            Err(Error::KeyNotFound)
        ));
        assert!(
            matches!(store.read("a", "y"), Err(Error::KeyNotFound)),
            "failed update left no key behind"
        );
        assert_eq!(store.delete("a", "x").unwrap(), dec("2"));
        assert!(matches!(store.read("a", "x"), Err(Error::KeyNotFound)));
    }

    #[test]
    fn compute_into_is_stored() {
        use crate::op::{Divide, Multiply};
        let store = Store::in_memory().unwrap();
        store.create("n", "g", &dec("0.5")).unwrap();
        let r = store
            .compute_into("n", Multiply, "g", &dec("10"), "out")
            .unwrap();
        assert_eq!(r, dec("5"));
        assert_eq!(store.read("n", "out").unwrap(), dec("5"));
        assert!(matches!(
            store.compute_into("n", Divide, "g", &dec("0"), "bad"),
            Err(Error::DivisionByZero)
        ));
        assert!(matches!(store.read("n", "bad"), Err(Error::KeyNotFound)));
    }
}
