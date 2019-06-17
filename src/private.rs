use anymap;
use std::collections::HashMap;
use std::any::{Any, TypeId};

type TraitRegistry = anymap::Map<dyn anymap::any::Any + Sync>;

lazy_static::lazy_static! {
    /// This is a global table of all the trait objects that can be cast into. 
    /// Each entry is a TraitImplTable, i.e. a table of the implementations of 
    /// a castable trait.
    static ref TRAIT_REGISTRY: TraitRegistry = {
        let mut master = TraitRegistry::new();
        for implable in inventory::iter::<TraitEntryBuilder> {
            (implable.insert)(&mut master);
        }
        master
    };
}

pub fn get_impl_table<DynTrait : ?Sized + 'static>() 
    -> Option<&'static TraitImplTable<DynTrait>>
{
    TRAIT_REGISTRY.get::<TraitImplTable<DynTrait>>()
}

/// For a castable trait, this is a table of the implementation of that trait.
pub struct TraitImplTable<DynTrait : ?Sized + 'static> {
    pub map: HashMap<TypeId, &'static ImplEntry<DynTrait>>
}

/// An entry in the table for a particular castable trait. Stores one 
/// implementation.
pub struct ImplEntry<DynTrait : ?Sized> {
    pub cast_box: fn(Box<Any>) -> Result<Box<DynTrait>, Box<Any>>,
    pub cast_mut: fn(&mut dyn Any) -> Option<&mut DynTrait>,
    pub cast_ref: fn(&dyn Any) -> Option<&DynTrait>,
    pub tid: TypeId
}

/// This is instantiated once for each castable trait. It describes how a trait
/// can insert itself into the global table.
pub struct TraitEntryBuilder {
    pub insert: fn(&mut TraitRegistry)
}

inventory::collect!(TraitEntryBuilder);
