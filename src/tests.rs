#![cfg(test)]

use std::any::Any;

mod traits {
    pub trait Foo {
        fn foo(&mut self) -> i64;
    }

    pub trait Bar {
        fn bar(&self) -> i64;
    }

    pub trait Baz {
        fn baz(self: Box<Self>) -> i64;
    }

    crate::register_trait!(Foo, Foo_Traitcast);
    crate::register_trait!(Bar, Bar_Traitcast);
    crate::register_trait!(Baz, Baz_Traitcast);
}

mod structs {
    use crate::tests::traits::{Foo, Bar, Baz};
    pub struct A {
        pub x: i64
    }

    pub struct B {
        pub y: i64
    }

    impl Foo for A {
        fn foo(&mut self) -> i64 {
            self.x += 1;
            self.x
        }
    }

    impl Bar for A {
        fn bar(&self) -> i64 {
            self.x
        }
    }

    impl Foo for B {
        fn foo(&mut self) -> i64 {
            self.y *= 2;
            self.y
        }
    }

    impl Baz for B {
        fn baz(self: Box<Self>) -> i64 {
            self.y
        }
    }

    crate::register_impl!(Foo, A);
    crate::register_impl!(Bar, A);
    crate::register_impl!(Foo, B);
    crate::register_impl!(Baz, B);
}

use traits::*;
use structs::*;

#[test]
fn test_traitcast() {
    let mut x: Box<Any> = Box::new(A { x: 0 });
    let mut y: Box<Any> = Box::new(B { y: 1 });

    {
        let x = crate::cast_ref::<dyn Bar>(&*x).unwrap();
        assert_eq!(x.bar(), 0);

        assert!(crate::cast_ref::<dyn Bar>(&*y).is_none());
    }

    {
        let x = crate::cast_mut::<dyn Foo>(&mut *x).unwrap();
        assert_eq!(x.foo(), 1);
        assert_eq!(x.foo(), 2);

        let y = crate::cast_mut::<dyn Foo>(&mut *y).unwrap();
        assert_eq!(y.foo(), 2);
        assert_eq!(y.foo(), 4);
    }

    {
        assert!(crate::cast_box::<dyn Baz>(x).is_err());

        let y = crate::cast_box::<dyn Baz>(y).unwrap();
        assert_eq!(y.baz(), 4);
    }
}
