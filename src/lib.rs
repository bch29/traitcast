/*!

#' Casting from `Any`

In the standard library, the std::any::Any trait comes with downcast methods 
which let you cast from an `Any` trait object to a concrete type.

```
# use std::any::Any;
let x: i32 = 7;
let y: &dyn Any = &x;

// Cast to i32 succeeds because x: i32
assert_eq!(y.downcast_ref::<i32>(), Some(&7));
// Cast to f32 fails
assert_eq!(y.downcast_ref::<f32>(), None);
```

However, it is not possible to downcast to a trait object.

```compile_fail
use std::any::Any;
trait Foo {
    fn foo(&self) -> i32;
}

struct A {
    x: i32
}

impl Foo for A {
    fn foo(&self) -> i32 {
        self.x
    }
}

let x = A { x: 7 };
let y: &dyn Any = &x;

// This cast is not possible, because it is only possible to cast to types that
// are Sized. Among other things, this precludes trait objects.
let z: Option<&dyn Foo> = y.downcast_ref();
```

## Traitcast

This library provides a way of casting from `dyn Any` to trait objects.

```
use std::any::Any;
# trait Foo { fn foo(&self) -> i32; }
# struct A { x: i32 }
# impl Foo for A { fn foo(&self) -> i32 { self.x } }

// Register the trait.
traitcast::register_trait!(Foo, Foo_Traitcast);

// For each struct that implements the trait, register the implementation.
traitcast::register_impl!(Foo, A);

fn main() {
    let mut x = A { x: 7 };

    {
        let y: &dyn Any = &x;
        // Test whether y is of a type that implements Foo.
        assert!(traitcast::implements_trait::<Foo>(y));
    }

    {
        let y: &dyn Any = &x;
        // Cast an immutable reference.
        let z: &dyn Foo = traitcast::cast_ref(y).unwrap();
        assert_eq!(z.foo(), 7);
    }

    {
        let y: &mut dyn Any = &mut x;
        // Cast a mutable reference
        let z: &mut dyn Foo = traitcast::cast_mut(y).unwrap();
        assert_eq!(z.foo(), 7);
    }

    {
        let y: Box<Any> = Box::new(x);
        // Cast a boxed reference
        let z: Box<dyn Foo> = traitcast::cast_box(y).unwrap();
        assert_eq!(z.foo(), 7);
    }
}
```

*/

use std::any::Any;

pub mod private;
#[cfg(test)]
pub mod tests;

use private::get_impl_table;

/// Tests whether the given value is castable to some trait object.
pub fn implements_trait<DynTrait>(x: &dyn Any) -> bool
    where DynTrait: CastableDynTrait + ?Sized + 'static
{
    cast_ref::<DynTrait>(x).is_some()
}

/// Tries to cast the given pointer to a dynamic trait object. This will always
/// return Err if the implementation of the trait, for the concrete type of x,
/// has not been registered.
pub fn cast_box<DynTrait>(x: Box<dyn Any>) 
    -> Result<Box<DynTrait>, Box<dyn Any>>
    where DynTrait: CastableDynTrait + ?Sized + 'static
{
    let trait_map = get_impl_table::<DynTrait>().expect(
        "Calling cast_box to cast into an unregistered trait object");

    // Must ensure we take the type id of what's in the box, not the type id of 
    // the box itself.
    let tid = (*x).type_id();

    let s = match trait_map.map.get(&tid) {
        Some(s) => s,
        None => return Err(x)
    };

    (s.cast_box)(x)
}

/// Tries to cast the given mutable reference to a dynamic trait object. This 
/// will always return None if the implementation of the trait, for the 
/// concrete type of x, has not been registered.
pub fn cast_mut<'a, DynTrait>(x: &'a mut dyn Any) -> Option<&'a mut DynTrait> 
    where DynTrait: CastableDynTrait + ?Sized + 'static
{
    let trait_map = get_impl_table::<DynTrait>().expect(
        "Calling cast_mut to cast into an unregistered trait object");
    let tid = {
        let x: &dyn Any = x;
        x.type_id()
    };
    let s = trait_map.map.get(&tid)?;
    (s.cast_mut)(x)
}

/// Tries to cast the given reference to a dynamic trait object. This will
/// always return None if the implementation of the trait, for the concrete
/// type of x, has not been registered.
pub fn cast_ref<'a, DynTrait>(x: &'a dyn Any) -> Option<&'a DynTrait> 
    where DynTrait: CastableDynTrait + ?Sized + 'static
{
    let trait_map = get_impl_table::<DynTrait>().expect(
        "Calling cast_ref to cast into an unregistered trait object");
    let tid = x.type_id();
    let s = trait_map.map.get(&tid)?;
    (s.cast_ref)(x)
}

/// Trait objects that can be cast into implement this trait.
pub unsafe trait CastableDynTrait {
    type ImplEntryWrapper: private::ImplEntryWrapper<Self>;
}

/// Register a trait to allow it to be cast into. Cannot cast from implementing 
/// structs unless register_impl is also called for that struct.
///
/// This macro may only be used on traits defined in the same module.
#[macro_export]
macro_rules! register_trait {
    ($trait:ident, $wrapper:ident) => {

        #[allow(non_camel_case_types)]
        pub struct $wrapper(pub $crate::private::ImplEntry<dyn $trait>);

        inventory::collect!($wrapper);

        unsafe impl $crate::private::ImplEntryWrapper<dyn $trait> for $wrapper {
            fn wrap(entry: $crate::private::ImplEntry<dyn $trait>) -> Self {
                $wrapper(entry)
            }
        }

        unsafe impl $crate::CastableDynTrait for dyn $trait {
            type ImplEntryWrapper = $wrapper;
        }

        inventory::submit! {
            $crate::private::TraitEntryBuilder {
                insert: |master| {
                    master.insert::<$crate::private::TraitImplTable<dyn $trait>>(
                        $crate::private::TraitImplTable {
                            map: inventory::iter::<$wrapper>
                                .into_iter()
                                .map(|x| (x.0.tid, &x.0))
                                .collect()
                        });
                }
            }
        }

    }
}

/// Register an implementation of a castable trait for a particular struct. The 
/// struct must implement the trait. This enables objects of this type to be
/// cast into dynamic trait references of this trait, via an Any pointer.
/// It is best not to invoke register_impl! multiple times for the same 
/// implementation. This will have no effect but to slightly slow down program 
/// load time.
///
/// This macro should only be used on structs defined in the same module.
#[macro_export]
macro_rules! register_impl {
    ($trait:ident, $struct:ident) => {
        inventory::submit! {
            let imp = $crate::private::ImplEntry::<dyn $trait> {
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
            };
            type IEW = <dyn $trait as $crate::CastableDynTrait>::ImplEntryWrapper;
            <IEW as $crate::private::ImplEntryWrapper<dyn $trait>>::wrap(imp)
        }
    }
}
