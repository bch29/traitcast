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
use traitcast::{TraitcastFrom, Traitcast};

// Extending `TraitcastFrom` is optional. This allows `Foo` objects themselves
// to be cast to other trait objects. If you do not extend `TraitcastFrom`,
// then Foo may only be cast into, not out of.
trait Foo: TraitcastFrom {
    fn foo(&self) -> i32;
}

trait Bar: TraitcastFrom {
    fn bar(&mut self) -> i32;
}

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
traitcast::traitcast!(struct A: Foo, Bar);

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

#[cfg(test)]
pub mod tests;

use std::any::Any;

pub use traitcast_core::TraitcastFrom;
pub use traitcast_core::traitcast;
use traitcast_core::inventory::build_registry;
use traitcast_core::Registry;

lazy_static::lazy_static! {
    /// This is a global table of all the trait objects that can be cast into.
    /// Each entry is a CastIntoTrait, i.e. a table of the implementations of a
    /// castable trait.
    static ref GLOBAL_REGISTRY: Registry =
        build_registry();
}

/// A convenience trait with a blanket implementation that adds methods to cast
/// from any trait that implements TraitcastFrom, to target with a static
/// lifetime.
pub trait Traitcast<To: ?Sized> {
    /// A convenience method that wraps the top-level `cast_ref` function.
    fn cast_ref(&self) -> Option<&To>;

    /// A convenience method that wraps the top-level `cast_mut` function.
    fn cast_mut(&mut self) -> Option<&mut To>;

    /// A convenience method that wraps the top-level `cast_box` function.
    fn cast_box(self: Box<Self>) -> Result<Box<To>, Box<dyn Any>>;
}

impl<From, To> Traitcast<To> for From
where
    From: TraitcastFrom + ?Sized,
    To: ?Sized + 'static,
{
    /// Tries to cast self to a different dynamic trait object. This will
    /// always return None if the implementation of the target trait, for the
    /// concrete type of self, has not been registered via
    /// `traitcast!`.
    fn cast_ref(&self) -> Option<&To> {
        cast_ref(self)
    }

    /// Tries to cast the self to a different dynamic trait object.  This will
    /// always return None if the implementation of the target trait, for the
    /// concrete type of self, has not been registered via
    /// `traitcast!`.
    fn cast_mut(&mut self) -> Option<&mut To> {
        cast_mut(self)
    }

    /// Tries to cast self to a boxed dynamic trait object. This will always
    /// return Err if the implementation of the target trait, for the concrete
    /// type of self, has not been registered via `traitcast!`.
    fn cast_box(self: Box<Self>) -> Result<Box<To>, Box<dyn Any>> {
        cast_box(self)
    }
}

/// Tests whether the given value is castable to some trait object. This will
/// always return `false` if the implementation of the target trait, for the
/// concrete type of x, has not been registered via `traitcast!`.
pub fn implements_trait<From, To>(x: &From) -> bool
where
    From: TraitcastFrom + ?Sized,
    To: ?Sized + 'static,
{
    cast_ref::<From, To>(x).is_some()
}

/// Tries to cast the given pointer to a dynamic trait object. This will always
/// return Err if the implementation of the target trait, for the concrete type
/// of x, has not been registered via `traitcast!`.
pub fn cast_box<From, To>(x: Box<From>) -> Result<Box<To>, Box<dyn Any>>
where
    From: TraitcastFrom + ?Sized,
    To: ?Sized + 'static,
{
    GLOBAL_REGISTRY
        .cast_into::<To>()
        .expect("Calling cast_box to cast into an unregistered trait object")
        .from_box(x)
}

/// Tries to cast the given mutable reference to a dynamic trait object. This
/// will always return None if the implementation of the target trait, for the
/// concrete type of x, has not been registered via `traitcast!`.
pub fn cast_mut<'a, From, To>(x: &'a mut From) -> Option<&'a mut To>
where
    From: TraitcastFrom + ?Sized,
    To: ?Sized + 'static,
{
    GLOBAL_REGISTRY
        .cast_into::<To>()
        .expect("Calling cast_mut to cast into an unregistered trait object")
        .from_mut(x)
}

/// Tries to cast the given reference to a dynamic trait object. This will
/// always return None if the implementation of the target trait, for the
/// concrete type of x, has not been registered via `traitcast!`.
pub fn cast_ref<'a, From, To>(x: &'a From) -> Option<&'a To>
where
    From: TraitcastFrom + ?Sized,
    To: ?Sized + 'static,
{
    GLOBAL_REGISTRY
        .cast_into::<To>()
        .expect("Calling cast_ref to cast into an unregistered trait object")
        .from_ref(x)
}
