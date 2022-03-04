use crate::ptr::IBase;
use crate::{base_object, DynObj, ObjArc, ObjWeak};
use std::cell::RefCell;

#[test]
fn uninhabited() {
    enum Void {}
    let a = ObjWeak::<Void>::new();
    assert!(a.upgrade().is_none());
    assert_eq!(a.weak_count(), 0);
}

#[test]
fn float_nan_ne() {
    let x = ObjArc::new(f32::NAN);
    assert_ne!(x, x);
    assert!(!(x == x));
}

#[test]
fn partial_eq() {
    struct TestPEq(RefCell<usize>);
    impl PartialEq for TestPEq {
        fn eq(&self, other: &TestPEq) -> bool {
            *self.0.borrow_mut() += 1;
            *other.0.borrow_mut() += 1;
            true
        }
    }
    let x = ObjArc::new(TestPEq(RefCell::new(0)));
    assert!(x == x);
    assert!(!(x != x));
    assert_eq!(*x.0.borrow(), 4);
}

#[test]
fn eq() {
    #[derive(Eq)]
    struct TestEq(RefCell<usize>);
    impl PartialEq for TestEq {
        fn eq(&self, other: &TestEq) -> bool {
            *self.0.borrow_mut() += 1;
            *other.0.borrow_mut() += 1;
            true
        }
    }
    let x = ObjArc::new(TestEq(RefCell::new(0)));
    assert!(x == x);
    assert!(!(x != x));
    assert_eq!(*x.0.borrow(), 0);
}

#[test]
fn weak_may_dangle() {
    fn hmm<'a>(val: &'a mut ObjWeak<&'a str>) -> ObjWeak<&'a str> {
        val.clone()
    }

    // Without #[may_dangle] we get:
    let mut val = ObjWeak::new();
    hmm(&mut val);
    //  ~~~~~~~~ borrowed value does not live long enough
    //
    // `val` dropped here while still borrowed
    // borrow might be used here, when `val` is dropped and runs the `Drop` code for type `std::sync::Weak`
}

#[test]
fn drop_sized() {
    struct SizedDrop<'a>(&'a RefCell<usize>);
    impl<'a> Drop for SizedDrop<'a> {
        fn drop(&mut self) {
            *self.0.borrow_mut() = 1;
        }
    }
    let val = RefCell::new(0);
    let x = ObjArc::new(SizedDrop(&val));
    assert_eq!(*x.0.borrow(), 0);

    std::mem::drop(x);
    assert_eq!(*val.borrow(), 1);
}

#[test]
fn drop_obj() {
    struct TestObj(ObjArc<RefCell<usize>>);
    impl Drop for TestObj {
        fn drop(&mut self) {
            *self.0.borrow_mut() = 1;
        }
    }
    base_object! { #![uuid(0x6e3178d1, 0xad1e, 0x4071, 0xaa82, 0xd732eefe118f)] impl TestObj }

    let val = ObjArc::new(RefCell::new(0));
    let x = ObjArc::new(TestObj(val.clone()));
    assert_eq!(*x.0.borrow(), 0);

    let x: ObjArc<DynObj<dyn IBase>> = ObjArc::coerce_obj(x);
    assert_eq!(*val.borrow(), 0);

    std::mem::drop(x);
    assert_eq!(*val.borrow(), 1);
}
