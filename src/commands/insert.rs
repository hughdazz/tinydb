use anyhow::{bail, Result};
use sqlparser::ast::{self, ObjectName};

use crate::{
    access::{heap::heap_insert, heaptuple::HeapTuple},
    catalog,
    encode::encode,
    errors::Error,
    storage::{rel::RelationData, BufferPool},
    Datums, Oid,
};

pub fn insert_into(
    buffer_pool: &mut BufferPool,
    db_data: &str,
    db_oid: &Oid,
    table_name: ObjectName,
    columns: Vec<ast::Ident>,
    source: Box<ast::Query>,
) -> Result<()> {
    let rel_name = table_name.0[0].to_string();
    let pg_class_rel = catalog::get_pg_class_relation(buffer_pool, db_data, db_oid, &rel_name)?;

    let rel = RelationData::open(
        pg_class_rel.oid,
        db_data,
        pg_class_rel.reltablespace,
        db_oid,
        &rel_name,
    );

    match source.body {
        ast::SetExpr::Values(values) => {
            let tuple_desc =
                catalog::tuple_desc_from_relation(buffer_pool, db_data, db_oid, &rel_name)?;

            let mut heap_values = Datums::default();

            // Iterate over all rows on insert to write new tuples.
            for row in &values.0 {
                if row.len() != columns.len() {
                    bail!("INSERT has more expressions than target columns");
                }

                // Iterate over relation attrs and try to find the value that is being inserted
                // for each attr. If the value does not exists a NULL value should be inserted
                // on tuple header t_bits array.
                for attr in &tuple_desc.attrs {
                    // TODO: Find a better way to lookup the attr value that is being inserted
                    let index = columns.iter().position(|ident| ident.value == attr.attname);
                    match index {
                        Some(index) => {
                            let value = &row[index];
                            match value {
                                ast::Expr::Value(value) => {
                                    encode(&mut heap_values, &value)?;
                                }
                                _ => bail!(Error::UnsupportedOperation(value.to_string())),
                            }
                        }
                        None => {
                            heap_values.push(None);
                        }
                    }
                }
            }

            heap_insert(buffer_pool, &rel, &mut HeapTuple::from_datums(heap_values)?)?;
        }
        _ => bail!(Error::UnsupportedOperation(source.to_string())),
    }

    Ok(())
}
