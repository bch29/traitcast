/*!
This module defines helper types for using `traitcast` along with the
`inventory` crate. Requires the `use_inventory` feature.
*/
use crate::{CastIntoTrait, ImplEntry, Registry};

/// Makes a trait registry by collecting EntryBuilders with the `inventory`
/// crate.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();
    for builder in inventory::iter::<EntryBuilder> {
        (builder.insert)(&mut reg);
    }
    reg
}

/// This is instantiated once for each castable trait. It describes how a trait
/// can insert itself into the global table.
pub struct EntryBuilder {
    pub insert: Box<Fn(&mut Registry)>,
}

impl EntryBuilder {
    /// Constructs a EntryBuilder for trait `To` by collecting ImplEntry<To> from
    /// `inventory`. If the table for `To` exists already, overwrites it.
    pub fn collecting_entries<To, Entry>() -> EntryBuilder
    where
        Entry: inventory::Collect + AsRef<ImplEntry<To>>,
        To: 'static + ?Sized,
    {
        use std::iter::FromIterator;
        EntryBuilder {
            insert: Box::new(|master| {
                master.insert(CastIntoTrait::from_iter(
                    inventory::iter::<Entry>
                        .into_iter()
                        .map(|x| x.as_ref().clone()),
                ))
            }),
        }
    }

    /// Constructs a trait builder that enters a single 'from' entry into the 
    /// table for a particular target.
    ///
    /// If the does not exist already, creates a new table. If it exists 
    /// already, modifies the existing table by inserting the new entry.
    pub fn inserting_entry<To>(entry: ImplEntry<To>) -> EntryBuilder
    where
        To: 'static + ?Sized,
    {
        EntryBuilder {
            insert: Box::new(move |master| {
                let table: &mut CastIntoTrait<To> = 
                    master.tables
                    .entry::<CastIntoTrait<To>>()
                    .or_insert(CastIntoTrait::new());

                table.map.insert(entry.tid, entry.clone());
            })
        }
    }
}

inventory::collect!(EntryBuilder);
