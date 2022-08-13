use crate::{
    access::{
        heap::heap_insert,
        heaptuple::{HeapTuple, HeapTupleHeader},
        tuple::TupleDesc,
    },
    storage::{
        bufpage::PageHeader,
        rel::{Relation, RelationData},
        BufferPool, PAGE_SIZE,
    },
    Oid,
};
use anyhow::Result;
use log::debug;

use super::{pg_attribute::PgAttribute, pg_class::PgClass};

/// Create a new cataloged heap relation.
pub fn heap_create(
    buffer: &mut BufferPool,
    db_data: &str,
    db_name: &str,
    rel_name: &str,
    new_rel_oid: Oid,
    tupledesc: &TupleDesc,
) -> Result<()> {
    // Create a new relation object for the new heap relation.
    let new_rel = RelationData::open(new_rel_oid, db_data, db_name, rel_name);

    // Now add tuples to pg_attribute for the attributes in our new relation.
    add_new_attribute_tuples(buffer, &new_rel, &tupledesc)?;

    // Open pg_class relation to store the new relation
    let pg_class = PgClass::relation(db_data, db_name);

    // Now create an entry in pg_class for the relation.
    add_new_relation_tuple(buffer, &pg_class, &new_rel)?;

    // Now that the new relation is already stored on pg_class, initialize the default page header
    // data
    initialize_default_page_header(buffer, &new_rel)?;

    Ok(())
}

/// Registers the new relation's schema by adding tuples to pg_attribute.
fn add_new_attribute_tuples(
    buffer: &mut BufferPool,
    rel: &Relation,
    tupledesc: &TupleDesc,
) -> Result<()> {
    let rel = rel.borrow();

    // Open pg_attribute relation to store the new relation attributes.
    let pg_attribute = PgAttribute::relation(&rel.locator.db_data, &rel.locator.db_name);

    // Now insert a new tuple on pg_attribute containing the new attributes information.
    for attr in &tupledesc.attrs {
        heap_insert(
            buffer,
            &pg_attribute,
            &mut HeapTuple {
                header: HeapTupleHeader::default(),
                data: bincode::serialize(&attr)?,
            },
        )?;
    }

    Ok(())
}

/// Registers the new relation in the catalogs by adding a tuple to pg_class. If the pg_class is
/// empty the buffer pool is used to alloc a new page on pg_class file and initialize the default
/// header values.
fn add_new_relation_tuple(
    buffer: &mut BufferPool,
    pg_class: &Relation,
    new_rel: &Relation,
) -> Result<()> {
    let new_rel = new_rel.borrow();

    // Initialize default page header of pg_class relation if needed.
    if pg_class.borrow_mut().smgr()?.borrow().size()? == 0 {
        initialize_default_page_header(buffer, pg_class)?;
    }

    // Now insert a new tuple on pg_class containing the new relation information.
    heap_insert(
        buffer,
        pg_class,
        &mut HeapTuple {
            header: HeapTupleHeader::default(),
            data: bincode::serialize(&PgClass {
                oid: new_rel.locator.oid,
                relname: new_rel.rel_name.clone(),
            })?,
        },
    )?;

    Ok(())
}

/// Initialize the default page header values on the given relation. The buffer pool is used to
/// alloc a new page on relation.
pub fn initialize_default_page_header(buffer: &mut BufferPool, rel: &Relation) -> Result<()> {
    let buf_id = buffer.alloc_buffer(rel)?;

    let mut data = bincode::serialize(&PageHeader::default())?;
    data.resize(PAGE_SIZE, u8::default());

    let page = buffer.get_page(&buf_id);
    page.borrow_mut().write_from_vec(data);

    buffer.flush_buffer(&buf_id)?;
    buffer.unpin_buffer(buf_id, true)?;

    debug!(
        "Initialized default pager header data for relation: {}",
        rel.borrow().rel_name
    );

    Ok(())
}
