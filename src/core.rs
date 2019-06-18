/*!
This module is a general interface to `traitcast` which does not rely on a
global registry. This makes it more flexible at the cost of having to create
a registry and pass it around. If you do not want to do that, use the root
`traitcast` module which provides a convenient global registry.
*/

use anymap;
use std::collections::HashMap;
use std::any::{Any, TypeId};

/// A registry defining how to cast into some set of traits.
pub struct Registry {
    tables: anymap::Map<dyn anymap::any::Any + Sync>
}

impl Registry {
    /// Makes a new, empty trait registry.
    pub fn new() -> Registry {
        Registry {
            tables: anymap::Map::new()
        }
    }

    /// Updates the table defining how to cast into the given trait.
    pub fn insert<DynTrait: ?Sized + 'static>(
        &mut self, table: CastIntoTrait<DynTrait>)
    {
        self.tables.insert(table);
    }

    /// Gets the table defining how to cast into the given trait.
    ///
    /// This method is designed to be chained with from_mut, from_ref or 
    /// from_box.
    ///
    /// # Examples
    /// ```text
    /// let x: &dyn Bar = ...;
    /// registry.cast_into::<Foo>()?.from_ref(x)
    ///
    /// let x: &mut dyn Bar = ...;
    /// registry.cast_into::<Foo>()?.from_mut(x)
    ///
    /// let x: Box<dyn Bar> = ...;
    /// registry.cast_into::<Foo>()?.from_box(x)
    /// ```
    pub fn cast_into<To>(&self) -> Option<&CastIntoTrait<To>>
        where To: ?Sized + 'static
    {
        self.tables.get::<CastIntoTrait<To>>()
    }
}

/// Provides methods for casting into the target trait object from other trait 
/// objects.
pub struct CastIntoTrait<DynTrait : ?Sized> {
    map: HashMap<TypeId, ImplEntry<DynTrait>>
}

impl<DynTrait: ?Sized> std::iter::FromIterator<ImplEntry<DynTrait>>
    for CastIntoTrait<DynTrait>
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = ImplEntry<DynTrait>>
    {
        CastIntoTrait {
            map: iter.into_iter().map(|x| (x.tid, x)).collect()
        }
    }
}

impl<To : ?Sized + 'static> CastIntoTrait<To> {
    /// Tries to cast the given reference to a dynamic trait object. This will
    /// always return None if the implementation of the target trait, for the 
    /// concrete type of x, has not been registered via `traitcast_to_impl!`.
    pub fn from_ref<'a, From>(&self, x: &'a From) -> Option<&'a To>
        where From: TraitcastFrom + ?Sized
    {
        let x = (*x).as_any_ref();
        let tid = x.type_id();
        let s = self.map.get(&tid)?;
        (s.cast_ref)(x)
    }

    /// Tries to cast the given mutable reference to a dynamic trait object. 
    /// This will always return None if the implementation of the target trait, 
    /// for the concrete type of x, has not been registered via 
    /// `traitcast_to_impl!`.
    pub fn from_mut<'a, From>(&self, x: &'a mut From) -> Option<&'a mut To> 
        where From: TraitcastFrom + ?Sized
    {
        let x = (*x).as_any_mut();
        let tid = (x as &dyn Any).type_id();
        let s = self.map.get(&tid)?;
        (s.cast_mut)(x)
    }

    /// Tries to cast the given pointer to a dynamic trait object. This will 
    /// always return Err if the implementation of the target trait, for the 
    /// concrete type of x, has not been registered via `traitcast_to_impl!`.
    pub fn from_box<From>(&self, x: Box<From>) 
        -> Result<Box<To>, Box<dyn Any>>
        where From: TraitcastFrom + ?Sized
    {
        let x = x.as_any_box();

        // Must ensure we take the type id of what's in the box, not the type 
        // id of the box itself.
        let tid = (*x).type_id();

        let s = match self.map.get(&tid) {
            Some(s) => s,
            None => return Err(x)
        };

        (s.cast_box)(x)
    }
}

/// An entry in the table for a particular castable trait. Stores methods to
/// cast into one particular struct that implements the trait.
pub struct ImplEntry<DynTrait : ?Sized> {
    pub cast_box: fn(Box<Any>) -> Result<Box<DynTrait>, Box<Any>>,
    pub cast_mut: fn(&mut dyn Any) -> Option<&mut DynTrait>,
    pub cast_ref: fn(&dyn Any) -> Option<&DynTrait>,
    pub tid: TypeId
}

/// Manual `Clone` impl to allow for unsized T.
impl<T: ?Sized> Clone for ImplEntry<T> {
    fn clone(&self) -> Self {
        ImplEntry {
            cast_box: self.cast_box,
            cast_mut: self.cast_mut,
            cast_ref: self.cast_ref,
            tid: self.tid
        }
    }
}

/// Constructs a `ImplEntry` for a trait and a concrete struct implementing 
/// that trait.
///
/// # Example
/// ```
/// # use traitcast::impl_entry;
/// # use traitcast::core::ImplEntry;
/// use std::fmt::Display;
/// let x: ImplEntry<Display> = impl_entry!(Display, i32);
/// ```
#[macro_export]
macro_rules! impl_entry {
    ($trait:ident, $struct:ident) => {
        $crate::core::ImplEntry::<dyn $trait> {
            cast_box: |x| {
                let x: Box<$struct> = x.downcast()?;
                let x: Box<dyn $trait> = x;
                Ok(x)
            },
            cast_mut: |x| {
                let x: &mut $struct = x.downcast_mut()?;
                let x: &mut dyn $trait = x;
                Some(x)
            },
            cast_ref: |x| {
                let x: &$struct = x.downcast_ref()?;
                let x: &dyn $trait = x;
                Some(x)
            },
            tid: std::any::TypeId::of::<$struct>()
        }
    }
}

/// Creates a struct named `$wrapper` which wraps `ImplEntry<dyn $trait>` for 
/// the given `$trait`. This is useful because it allows implementing traits on 
/// the `ImplEntry<dyn $trait>` from external modules. This is an 
/// implementation detail of `traitcast_to_trait!`.
#[macro_export]
macro_rules! impl_entry_wrapper {
    ($trait:ident, $wrapper:ident) => {
        #[allow(non_camel_case_types)]
        pub struct $wrapper(pub $crate::core::ImplEntry<dyn $trait>);

        impl std::convert::From<$crate::core::ImplEntry<dyn $trait>> 
            for $wrapper 
        {
            fn from(x: $crate::core::ImplEntry<dyn $trait>) -> Self {
                $wrapper(x)
            }
        }

        impl std::convert::AsRef<$crate::core::ImplEntry<dyn $trait>>
            for $wrapper
        {
            fn as_ref(&self) -> &$crate::core::ImplEntry<dyn $trait> {
                &self.0
            }
        }
    }
}


/// Subtraits of `TraitcastFrom` may be cast into `dyn Any`, and thus may be 
/// cast into any other castable dynamic trait object, too. This is blanket 
/// implemented for all sized types with static lifetimes.
pub trait TraitcastFrom {
    /// Cast to an immutable reference to a trait object.
    fn as_any_ref(&self) -> &dyn Any;

    /// Cast to a mutable reference to a trait object.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Cast to a boxed reference to a trait object.
    fn as_any_box(self: Box<Self>) -> Box<dyn Any>;

    /// Get the trait object's dynamic type id.
    fn type_id(&self) -> std::any::TypeId {
        self.as_any_ref().type_id()
    }
}

/// Blanket implementation that automatically implements TraitcastFrom for most 
/// user-defined types.
impl<T> TraitcastFrom for T where T: Sized + 'static {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl TraitcastFrom for dyn Any {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

