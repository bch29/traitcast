Traitcast
---------

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

