/*!
This module defines helper types for using `traitcast` along with the 
`inventory` crate.
*/
use crate::core::{Registry, ImplEntry, CastIntoTrait};

/// Makes a trait registry by collecting TraitBuilders with the 
/// `inventory` crate.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();
    for builder in inventory::iter::<TraitBuilder> {
        (builder.insert)(&mut reg);
    }
    reg
}

/// This is instantiated once for each castable trait. It describes how a trait
/// can insert itself into the global table.
pub struct TraitBuilder {
    pub insert: fn(&mut Registry)
}

impl TraitBuilder {
    /// Constructs a TraitBuilder for trait To by collecting ImplEntry<To> from 
    /// `inventory`.
    pub fn collecting_entries<To, Entry>() -> TraitBuilder
        where Entry: inventory::Collect + AsRef<ImplEntry<To>>,
            To: 'static + ?Sized
    {
        use std::iter::FromIterator;
        TraitBuilder {
        insert: |master| master.insert(
            CastIntoTrait::from_iter(
                inventory::iter::<Entry>.into_iter()
                .map(|x| x.as_ref().clone())))
        }
    }
}

inventory::collect!(TraitBuilder);

