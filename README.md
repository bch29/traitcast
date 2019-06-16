Traitcast
---------

## Casting from `Any`

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
traitcast::register_impl!(Foo, Foo_Traitcast, A);

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

