/*!

## Casting from `Any`

In the standard library, the std::any::Any trait comes with downcast methods 
which let you cast from an `Any` trait object to a concrete type.

```rust
let x: i32 = 7;
let y: &dyn std::any::Any = &x;

// Cast to i32 succeeds because x: i32
assert_eq!(y.downcast_ref::<i32>(), Some(&7));
// Cast to f32 fails
assert_eq!(y.downcast_ref::<f32>(), None);
```

However, it is not possible to downcast to a trait object.

```compile_fail
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
let y: &dyn std::any::Any = &x;

// This cast is not possible, because it is only possible to cast to types that
// are Sized. Among other things, this precludes trait objects.
let z: Option<&dyn Foo> = y.downcast_ref();
```

## Traitcast

This library provides a way of casting between different trait objects.

```rust
use traitcast::{TraitcastFrom, TraitcastTo, Traitcast};

// Extending `TraitcastFrom` is optional. This allows `Foo` objects themselves 
// to be cast to other trait objects. If you do not extend `TraitcastFrom`, 
// then Foo may only be cast into, not out of.
trait Foo: TraitcastFrom {
    fn foo(&self) -> i32; 
}

// Invoking `traitcast_to_trait!` implements TraitcastTo for this Foo, allowing 
// other trait objects to be cast into Foo trait objects.
traitcast::traitcast_to_trait!(Foo, Foo_Traitcast);

trait Bar: TraitcastFrom {
    fn bar(&mut self) -> i32;
}

traitcast::traitcast_to_trait!(Bar, Bar_Traitcast);

struct A {
    x: i32 
}

// No implementation of TraitcastFrom is necessary, because it is covered by 
// the blanket impl for any sized type with a static lifetime.
impl Foo for A {
    fn foo(&self) -> i32 {
        self.x 
    } 
}

impl Bar for A {
    fn bar(&mut self) -> i32 {
        self.x *= 2;
        self.x
    }
}

// Register the traits.

// For each struct that implements each trait, register the implementation.
traitcast::traitcast_to_impl!(Foo, A);
traitcast::traitcast_to_impl!(Bar, A);

fn main() {
    let mut x = A { x: 7 };

    {
        let x: &dyn Foo = &x;
        // Test whether x is of a type that implements Bar.
        assert!(traitcast::implements_trait::<dyn Foo, dyn Bar>(x));
    }

    {
        let x: &dyn Bar = &x;
        // Cast an immutable reference using the `cast_ref` method (via the 
        // `Traitcast` trait, which is blanket implemented for all pairs of 
        // traits that may be cast between).
        let x: &dyn Foo = x.cast_ref().unwrap();
        assert_eq!(x.foo(), 7);

        // We can also cast using the top-level `cast_ref` function, which can
        // be more convenient when type arguments cannot be inferred.
        assert!(traitcast::cast_ref::<dyn Foo, dyn Bar>(x).is_some());
    }

    {
        let x: &mut dyn Foo = &mut x;
        // Cast a mutable reference using the `cast_mut` method 
        let x: &mut dyn Bar = x.cast_mut().unwrap();
        assert_eq!(x.bar(), 14);
    }

    {
        // We can cast from `Any` too!
        let y: Box<dyn std::any::Any> = Box::new(x);
        // Cast a boxed reference
        let z: Box<dyn Foo> = y.cast_box().unwrap();
        assert_eq!(z.foo(), 14);
    }
}
```

*/

use std::any::Any;

pub mod private;
#[cfg(test)]
pub mod tests;

use private::get_impl_table;

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

/// A convenience trait with a blanket implementation that adds methods to cast
/// from any trait that implements TraitcastFrom, to any trait that implements
/// TraitcastTo.
pub trait Traitcast<To: ?Sized> {
    /// A convenience method that wraps the top-level `cast_ref` function.
    fn cast_ref(&self) -> Option<&To>;

    /// A convenience method that wraps the top-level `cast_mut` function.
    fn cast_mut(&mut self) -> Option<&mut To>;

    /// A convenience method that wraps the top-level `cast_box` function.
    fn cast_box(self: Box<Self>) -> Result<Box<To>, Box<dyn Any>>;
}

impl<From, To> Traitcast<To> for From
    where From: TraitcastFrom + ?Sized,
          To: TraitcastTo + ?Sized + 'static
{
    /// Tries to cast self to a different dynamic trait object. This will
    /// always return None if the implementation of the target trait, for the 
    /// concrete type of self, has not been registered via 
    /// `traitcast_to_impl!`.
    fn cast_ref(&self) -> Option<&To> {
        cast_ref(self)
    }

    /// Tries to cast the self to a different dynamic trait object.  This will
    /// always return None if the implementation of the target trait, for the
    /// concrete type of self, has not been registered via 
    /// `traitcast_to_impl!`.
    fn cast_mut(&mut self) -> Option<&mut To> {
        cast_mut(self)
    }

    /// Tries to cast self to a boxed dynamic trait object. This will always 
    /// return Err if the implementation of the target trait, for the concrete 
    /// type of self, has not been registered via `traitcast_to_impl!`.
    fn cast_box(self: Box<Self>) -> Result<Box<To>, Box<dyn Any>> {
        cast_box(self)
    }
}

/// Tests whether the given value is castable to some trait object. This will
/// always return `false` if the implementation of the target trait, for the
/// concrete type of x, has not been registered via `traitcast_to_impl!`.
pub fn implements_trait<From, To>(x: &From) -> bool
    where From: TraitcastFrom + ?Sized,
          To: TraitcastTo + ?Sized + 'static
{
    cast_ref::<From, To>(x).is_some()
}

/// Tries to cast the given pointer to a dynamic trait object. This will always
/// return Err if the implementation of the target trait, for the concrete type 
/// of x, has not been registered via `traitcast_to_impl!`.
pub fn cast_box<From, To>(x: Box<From>) 
    -> Result<Box<To>, Box<dyn Any>>
    where From: TraitcastFrom + ?Sized,
          To: TraitcastTo + ?Sized + 'static
{
    let trait_map = get_impl_table::<To>().expect(
        "Calling cast_box to cast into an unregistered trait object");

    let x = x.as_any_box();

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
/// will always return None if the implementation of the target trait, for the 
/// concrete type of x, has not been registered via `traitcast_to_impl!`.
pub fn cast_mut<'a, From, To>(x: &'a mut From) -> Option<&'a mut To> 
    where From: TraitcastFrom + ?Sized,
          To: TraitcastTo + ?Sized + 'static
{
    let trait_map = get_impl_table::<To>().expect(
        "Calling cast_mut to cast into an unregistered trait object");
    let x = (*x).as_any_mut();
    let tid = (x as &dyn Any).type_id();
    let s = trait_map.map.get(&tid)?;
    (s.cast_mut)(x)
}

/// Tries to cast the given reference to a dynamic trait object. This will
/// always return None if the implementation of the target trait, for the 
/// concrete type of x, has not been registered via `traitcast_to_impl!`.
pub fn cast_ref<'a, From, To>(x: &'a From) -> Option<&'a To> 
    where From: TraitcastFrom + ?Sized,
          To: TraitcastTo + ?Sized + 'static
{
    let trait_map = get_impl_table::<To>().expect(
        "Calling cast_ref to cast into an unregistered trait object");
    let x = (*x).as_any_ref();
    let tid = x.type_id();
    let s = trait_map.map.get(&tid)?;
    (s.cast_ref)(x)
}

/// Trait objects that can be cast into implement this trait.
pub unsafe trait TraitcastTo {
    type ImplEntryWrapper: From<private::ImplEntry<Self>>;
}

/// Register a trait to allow it to be cast into. Cannot cast from implementing 
/// structs unless `traitcast_to_impl!` is also invoked for that struct.
///
/// This macro may only be used on traits defined in the same module.
#[macro_export]
macro_rules! traitcast_to_trait {
    ($trait:ident, $wrapper:ident) => {

        #[allow(non_camel_case_types)]
        pub struct $wrapper(pub $crate::private::ImplEntry<dyn $trait>);

        inventory::collect!($wrapper);

        impl std::convert::From<$crate::private::ImplEntry<dyn $trait>> 
            for $wrapper 
        {
            fn from(x: $crate::private::ImplEntry<dyn $trait>) -> Self {
                $wrapper(x)
            }
        }

        unsafe impl $crate::TraitcastTo for dyn $trait {
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
/// It is best not to invoke traitcast_to_impl! multiple times for the same 
/// implementation. This will have no effect but to slightly slow down program 
/// load time.
///
/// This macro should only be used on structs defined in the same module.
#[macro_export]
macro_rules! traitcast_to_impl {
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
            type IEW = <dyn $trait as $crate::TraitcastTo>::ImplEntryWrapper;
            IEW::from(imp)
        }
    }
}
