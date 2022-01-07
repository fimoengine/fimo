use crate::ObjWeak;

#[test]
fn uninhabited() {
    enum Void {}
    let a = ObjWeak::<Void>::new();
    assert!(a.upgrade().is_none());
}
