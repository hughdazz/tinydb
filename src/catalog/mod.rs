use anyhow::{bail, Result};
use std::path::Path;

use crate::{
    access::{heap::heap_iter, tuple::TupleDesc},
    new_object_id,
    storage::BufferPool,
    Oid,
};

use self::{pg_attribute::PgAttribute, pg_class::PgClass};

pub mod heap;
pub mod pg_attribute;
pub mod pg_class;
pub mod pg_database;
pub mod pg_tablespace;

/// Genereate a new relation oid that is unique within the database of the given db data.
pub fn new_relation_oid(db_data: &str, db_name: &str) -> Oid {
    let dbpath = Path::new(db_data).join(db_name);

    loop {
        let oid = new_object_id();
        if !dbpath.join(oid.to_string()).exists() {
            return oid;
        }
    }
}

/// Errors related with system catalog relation operations.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("relation {0} does not exist")]
    RelationNotFound(String),
}

/// Return the tuple description of the given relation name.
pub fn tuple_desc_from_relation(
    buffer_pool: &mut BufferPool,
    db_data: &str,
    db_name: &str,
    rel_name: &str,
) -> Result<TupleDesc> {
    let pg_attribute = PgAttribute::relation(db_data, db_name);

    let rel_oid = get_oid_relation(buffer_pool, db_data, db_name, rel_name)?;

    let mut attributes = Vec::new();

    heap_iter(buffer_pool, &pg_attribute, |tuple| -> Result<()> {
        let attr = bincode::deserialize::<PgAttribute>(&tuple.data)?;
        if attr.attrelid == rel_oid {
            attributes.push(attr);
        }

        Ok(())
    })?;

    Ok(TupleDesc { attrs: attributes })
}

/// Return the oid of the given relation name.
pub fn get_oid_relation(
    buffer_pool: &mut BufferPool,
    db_data: &str,
    db_name: &str,
    rel_name: &str,
) -> Result<Oid> {
    let pg_class_rel = PgClass::relation(db_data, db_name);

    let mut oid = None;

    heap_iter(buffer_pool, &pg_class_rel, |tuple| -> Result<()> {
        // Do nothing if the oid is already founded.
        if oid.is_none() {
            let pg_class = bincode::deserialize::<PgClass>(&tuple.data)?;
            if pg_class.relname == rel_name {
                oid = Some(pg_class.oid);
            }
        }
        Ok(())
    })?;

    match oid {
        Some(oid) => Ok(oid),
        None => bail!(Error::RelationNotFound(rel_name.to_string())),
    }
}
