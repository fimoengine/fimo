#![feature(generic_nonzero)]
#![feature(allocator_api)]
use std::{
    fmt::Debug,
    mem::swap,
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};

// Copied from the `vec` tests of the Rust `alloc` crate.
use fimo_std::{array_list, array_list::ArrayList};

struct DropCounter<'a> {
    count: &'a mut u32,
}

impl Drop for DropCounter<'_> {
    fn drop(&mut self) {
        *self.count += 1;
    }
}

#[test]
fn test_double_drop() {
    struct TwoArr<T> {
        x: ArrayList<T>,
        y: ArrayList<T>,
    }

    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = TwoArr {
            x: ArrayList::new(),
            y: ArrayList::new(),
        };
        tv.x.push(DropCounter {
            count: &mut count_x,
        })
        .unwrap();
        tv.y.push(DropCounter {
            count: &mut count_y,
        })
        .unwrap();

        // If ArrayList had a drop flag, here is where it would be zeroed.
        // Instead, it should rely on its internal state to prevent
        // doing anything significant when dropped multiple times.
        drop(tv.x);

        // Here tv goes out of scope, tv.y should be dropped, but not tv.x.
    }

    assert_eq!(count_x, 1);
    assert_eq!(count_y, 1);
}

#[test]
fn test_reserve() {
    let mut v = ArrayList::new();
    assert_eq!(v.capacity(), 0);

    v.reserve(2).unwrap();
    assert!(v.capacity() >= 2);

    for i in 0..16 {
        v.push(i).unwrap();
    }

    assert!(v.capacity() >= 16);
    v.reserve(16).unwrap();
    assert!(v.capacity() >= 32);

    v.push(16).unwrap();

    v.reserve(16).unwrap();
    assert!(v.capacity() >= 33);
}

#[test]
fn test_indexing() {
    let v: ArrayList<isize> = array_list![10, 20].unwrap();
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    let mut x: usize = 0;
    assert_eq!(v[x], 10);
    assert_eq!(v[x + 1], 20);
    x += 1;
    assert_eq!(v[x], 20);
    assert_eq!(v[x - 1], 10);
}

#[test]
fn test_debug_fmt() {
    let arr1: ArrayList<isize> = array_list![].unwrap();
    assert_eq!("[]", format!("{:?}", arr1));

    let arr2 = array_list![0, 1].unwrap();
    assert_eq!("[0, 1]", format!("{:?}", arr2));

    let slice: &[isize] = &[4, 5];
    assert_eq!("[4, 5]", format!("{slice:?}"));
}

#[test]
fn test_push() {
    let mut a = array_list![].unwrap();
    a.push(1).unwrap();
    assert_eq!(a, [1]);
    a.push(2).unwrap();
    assert_eq!(a, [1, 2]);
    a.push(3).unwrap();
    assert_eq!(a, [1, 2, 3]);
}

#[test]
fn test_extend() {
    let mut v = ArrayList::new();
    let mut w = ArrayList::new();

    v.extend(w.clone());
    assert_eq!(v, &[]);

    v.extend(0..3);
    for i in 0..3 {
        w.push(i).unwrap();
    }

    assert_eq!(v, w);

    v.extend(3..10);
    for i in 3..10 {
        w.push(i).unwrap();
    }

    assert_eq!(v, w);

    v.extend(w.clone()); // specializes to `append`
    assert!(v.iter().eq(w.iter().chain(w.iter())));

    // Zero sized types
    #[derive(PartialEq, Debug)]
    struct Foo;

    let mut a = ArrayList::new();
    let b = array_list![Foo, Foo].unwrap();

    a.extend(b);
    assert_eq!(a, &[Foo, Foo]);

    // Double drop
    let mut count_x = 0;
    {
        let mut x = ArrayList::new();
        let y = array_list![DropCounter {
            count: &mut count_x,
        }]
        .unwrap();
        x.extend(y);
    }
    assert_eq!(count_x, 1);
}

#[test]
fn test_extend_ref() {
    let mut v = array_list![1, 2].unwrap();
    v.extend(&[3, 4, 5]);

    assert_eq!(v.len(), 5);
    assert_eq!(v, [1, 2, 3, 4, 5]);

    let w = array_list![6, 7].unwrap();
    v.extend(&w);

    assert_eq!(v.len(), 7);
    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_slice_from_ref() {
    let values = array_list![1, 2, 3, 4, 5].unwrap();
    let slice = &values[1..3];

    assert_eq!(slice, [2, 3]);
}

#[test]
fn test_slice_from_mut() {
    let mut values = array_list![1, 2, 3, 4, 5].unwrap();
    {
        let slice = &mut values[2..];
        assert!(slice == [3, 4, 5]);
        for p in slice {
            *p += 2;
        }
    }

    assert!(values == [1, 2, 5, 6, 7]);
}

#[test]
fn test_slice_to_mut() {
    let mut values = array_list![1, 2, 3, 4, 5].unwrap();
    {
        let slice = &mut values[..2];
        assert!(slice == [1, 2]);
        for p in slice {
            *p += 1;
        }
    }

    assert!(values == [2, 3, 3, 4, 5]);
}

#[test]
fn test_split_at_mut() {
    let mut values = array_list![1, 2, 3, 4, 5].unwrap();
    {
        let (left, right) = values.split_at_mut(2);
        {
            let left: &[_] = left;
            assert!(&left[..left.len()] == &[1, 2]);
        }
        for p in left {
            *p += 1;
        }

        {
            let right: &[_] = right;
            assert!(&right[..right.len()] == &[3, 4, 5]);
        }
        for p in right {
            *p += 2;
        }
    }

    assert_eq!(values, [2, 3, 5, 6, 7]);
}

#[test]
fn test_clone() {
    let v: ArrayList<i32> = array_list![].unwrap();
    let w = array_list![1, 2, 3].unwrap();

    assert_eq!(v, v.clone());

    let z = w.clone();
    assert_eq!(w, z);
    // they should be disjoint in memory.
    assert!(w.as_ptr() != z.as_ptr());
}

#[test]
fn test_clone_from() {
    let mut v = array_list![].unwrap();
    let three: ArrayList<Box<_>> = array_list![Box::new(1), Box::new(2), Box::new(3)].unwrap();
    let two: ArrayList<Box<_>> = array_list![Box::new(4), Box::new(5)].unwrap();
    // zero, long
    v.clone_from(&three);
    assert_eq!(v, three);

    // equal
    v.clone_from(&three);
    assert_eq!(v, three);

    // long, short
    v.clone_from(&two);
    assert_eq!(v, two);

    // short, long
    v.clone_from(&three);
    assert_eq!(v, three);
}

#[test]
fn test_retain() {
    let mut vec = array_list![1, 2, 3, 4].unwrap();
    vec.retain(|&x| x % 2 == 0).unwrap();
    assert_eq!(vec, [2, 4]);
}

#[test]
fn test_retain_predicate_order() {
    for to_keep in [true, false] {
        let mut number_of_executions = 0;
        let mut vec = array_list![1, 2, 3, 4].unwrap();
        let mut next_expected = 1;
        vec.retain(|&x| {
            assert_eq!(next_expected, x);
            next_expected += 1;
            number_of_executions += 1;
            to_keep
        })
        .unwrap();
        assert_eq!(number_of_executions, 4);
    }
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_pred_panic_with_hole() {
    let v = (0..5).map(Rc::new).collect::<ArrayList<_>>();
    catch_unwind(AssertUnwindSafe(|| {
        let mut v = v.clone();
        v.retain(|r| match **r {
            0 | 2 => true,
            1 => false,
            _ => panic!(),
        })
        .unwrap();
    }))
    .unwrap_err();
    // Everything is dropped when predicate panicked.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_pred_panic_no_hole() {
    let v = (0..5).map(Rc::new).collect::<ArrayList<_>>();
    catch_unwind(AssertUnwindSafe(|| {
        let mut v = v.clone();
        v.retain(|r| if let 0..=2 = **r { true } else { panic!() })
            .unwrap();
    }))
    .unwrap_err();
    // Everything is dropped when predicate panicked.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_retain_drop_panic() {
    struct Wrap(Rc<i32>);

    impl Drop for Wrap {
        fn drop(&mut self) {
            if *self.0 == 3 {
                panic!();
            }
        }
    }

    let v = (0..5).map(Rc::new).collect::<ArrayList<_>>();
    catch_unwind(AssertUnwindSafe(|| {
        let mut v = v.iter().map(|r| Wrap(r.clone())).collect::<ArrayList<_>>();
        v.retain(|w| match *w.0 {
            0 => true,
            1 => false,
            2 => true,
            3 => false, // Drop panic.
            _ => true,
        })
        .unwrap();
    }))
    .unwrap_err();
    // Other elements are dropped when `drop` of one element panicked.
    // The panicked wrapper also has its Rc dropped.
    assert!(v.iter().all(|r| Rc::strong_count(r) == 1));
}

#[test]
fn test_retain_maybeuninits() {
    // This test aimed to be run under miri.
    use core::mem::MaybeUninit;
    let mut vec: ArrayList<_> = [1i32, 2, 3, 4]
        .map(|v| MaybeUninit::new(array_list![v].unwrap()))
        .into();
    vec.retain(|x| {
        // SAFETY: Retain must visit every element of ArrayList in original order and exactly once.
        // Our values is initialized at creation of ArrayList.
        let v = unsafe { x.assume_init_ref()[0] };
        if v & 1 == 0 {
            return true;
        }
        // SAFETY: Value is initialized.
        // Value wouldn't be dropped by `ArrayList::retain`
        // because `MaybeUninit` doesn't drop content.
        drop(unsafe { x.assume_init_read() });
        false
    })
    .unwrap();
    let vec: ArrayList<i32> = vec
        .into_iter()
        // Safety:
        .map(|x| unsafe {
            // SAFETY: All values dropped in retain predicate must be removed by
            // `ArrayList::retain`. Remaining values are initialized.
            x.assume_init()[0]
        })
        .collect();
    assert_eq!(vec, [2, 4]);
}

#[test]
fn zero_sized_values() {
    let mut v = ArrayList::new();
    assert_eq!(v.len(), 0);
    v.push(()).unwrap();
    assert_eq!(v.len(), 1);
    v.push(()).unwrap();
    assert_eq!(v.len(), 2);
    assert_eq!(v.pop_back(), Some(()));
    assert_eq!(v.pop_back(), Some(()));
    assert_eq!(v.pop_back(), None);

    assert_eq!(v.iter().count(), 0);
    v.push(()).unwrap();
    assert_eq!(v.iter().count(), 1);
    v.push(()).unwrap();
    assert_eq!(v.iter().count(), 2);

    for &() in &v {}

    assert_eq!(v.iter_mut().count(), 2);
    v.push(()).unwrap();
    assert_eq!(v.iter_mut().count(), 3);
    v.push(()).unwrap();
    assert_eq!(v.iter_mut().count(), 4);

    for &mut () in &mut v {}
    // Safety:
    unsafe {
        v.set_len(0);
    }
    assert_eq!(v.iter_mut().count(), 0);
}

#[test]
fn test_partition() {
    assert_eq!(
        [].into_iter().partition(|x: &i32| *x < 3),
        (array_list![].unwrap(), array_list![].unwrap())
    );
    assert_eq!(
        [1, 2, 3].into_iter().partition(|x| *x < 4),
        (array_list![1, 2, 3].unwrap(), array_list![].unwrap())
    );
    assert_eq!(
        [1, 2, 3].into_iter().partition(|x| *x < 2),
        (array_list![1].unwrap(), array_list![2, 3].unwrap())
    );
    assert_eq!(
        [1, 2, 3].into_iter().partition(|x| *x < 0),
        (array_list![].unwrap(), array_list![1, 2, 3].unwrap())
    );
}

#[test]
fn test_zip_unzip() {
    let z1 = array_list![(1, 4), (2, 5), (3, 6)].unwrap();

    let (left, right): (ArrayList<_>, ArrayList<_>) = z1.iter().cloned().unzip();

    assert_eq!((1, 4), (left[0], right[0]));
    assert_eq!((2, 5), (left[1], right[1]));
    assert_eq!((3, 6), (left[2], right[2]));
}

#[test]
fn test_cmp() {
    let x: &[isize] = &[1, 2, 3, 4, 5];
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);

    let x: ArrayList<isize> = array_list![1, 2, 3, 4, 5].unwrap();
    let cmp: &[isize] = &[1, 2, 3, 4, 5];
    assert_eq!(&x[..], cmp);
    let cmp: &[isize] = &[3, 4, 5];
    assert_eq!(&x[2..], cmp);
    let cmp: &[isize] = &[1, 2, 3];
    assert_eq!(&x[..3], cmp);
    let cmp: &[isize] = &[2, 3, 4];
    assert_eq!(&x[1..4], cmp);
}

#[test]
fn test_vec_truncate_drop() {
    static mut DROPS: u32 = 0;
    struct Elem(#[allow(dead_code)] i32);
    impl Drop for Elem {
        fn drop(&mut self) {
            // Safety:
            unsafe {
                DROPS += 1;
            }
        }
    }

    let mut v = array_list![Elem(1), Elem(2), Elem(3), Elem(4), Elem(5)].unwrap();
    // Safety:
    assert_eq!(unsafe { DROPS }, 0);
    v.truncate(3);
    // Safety:
    assert_eq!(unsafe { DROPS }, 2);
    v.truncate(0);
    // Safety:
    assert_eq!(unsafe { DROPS }, 5);
}

#[test]
#[should_panic]
fn test_vec_truncate_fail() {
    struct BadElem(i32);
    impl Drop for BadElem {
        fn drop(&mut self) {
            let BadElem(ref mut x) = *self;
            if *x == 0xbadbeef {
                panic!("BadElem panic: 0xbadbeef")
            }
        }
    }

    let mut v = array_list![BadElem(1), BadElem(2), BadElem(0xbadbeef), BadElem(4)].unwrap();
    v.truncate(0);
}

#[test]
fn test_index() {
    let vec = array_list![1, 2, 3].unwrap();
    assert!(vec[1] == 2);
}

#[test]
#[should_panic]
fn test_index_out_of_bounds() {
    let vec = array_list![1, 2, 3].unwrap();
    let _ = vec[3];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_1() {
    let x = array_list![1, 2, 3, 4, 5].unwrap();
    let _ = &x[!0..];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_2() {
    let x = array_list![1, 2, 3, 4, 5].unwrap();
    let _ = &x[..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_3() {
    let x = array_list![1, 2, 3, 4, 5].unwrap();
    let _ = &x[!0..4];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_4() {
    let x = array_list![1, 2, 3, 4, 5].unwrap();
    let _ = &x[1..6];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds_5() {
    let x = array_list![1, 2, 3, 4, 5].unwrap();
    let _ = &x[3..2];
}

#[test]
#[should_panic]
fn test_swap_remove_empty() {
    let mut vec = ArrayList::<i32>::new();
    vec.swap_remove(0).unwrap();
}

#[test]
fn test_move_items() {
    let vec = array_list![1, 2, 3].unwrap();
    let mut vec2 = array_list![].unwrap();
    for i in vec {
        vec2.push(i).unwrap();
    }
    assert_eq!(vec2, [1, 2, 3]);
}

#[test]
fn test_move_items_reverse() {
    let vec = array_list![1, 2, 3].unwrap();
    let mut vec2 = array_list![].unwrap();
    for i in vec.into_iter().rev() {
        vec2.push(i).unwrap();
    }
    assert_eq!(vec2, [3, 2, 1]);
}

#[test]
fn test_move_items_zero_sized() {
    let vec = array_list![(), (), ()].unwrap();
    let mut vec2 = array_list![].unwrap();
    for i in vec {
        vec2.push(i).unwrap();
    }
    assert_eq!(vec2, [(), (), ()]);
}

#[test]
fn test_into_boxed_slice() {
    let xs = array_list![1, 2, 3].unwrap();
    let ys = xs.into_boxed_slice().unwrap();
    assert_eq!(&*ys, [1, 2, 3]);
}

#[test]
fn test_append() {
    let mut vec = array_list![1, 2, 3].unwrap();
    let mut vec2 = array_list![4, 5, 6].unwrap();
    vec.append(&mut vec2).unwrap();
    assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
    assert_eq!(vec2, []);
}

#[test]
fn test_split_off() {
    let mut vec = array_list![1, 2, 3, 4, 5, 6].unwrap();
    let orig_ptr = vec.as_ptr();
    let orig_capacity = vec.capacity();

    let split_off = vec.split_off(4).unwrap();
    assert_eq!(vec, [1, 2, 3, 4]);
    assert_eq!(split_off, [5, 6]);
    assert_eq!(vec.capacity(), orig_capacity);
    assert_eq!(vec.as_ptr(), orig_ptr);
}

#[test]
fn test_split_off_take_all() {
    // Allocate enough capacity that we can tell whether the split-off vector's
    // capacity is based on its size, or (incorrectly) on the original capacity.
    let mut vec = ArrayList::with_capacity(1000).unwrap();
    vec.extend([1, 2, 3, 4, 5, 6]);
    let orig_ptr = vec.as_ptr();
    let orig_capacity = vec.capacity();

    let split_off = vec.split_off(0).unwrap();
    assert_eq!(vec, []);
    assert_eq!(split_off, [1, 2, 3, 4, 5, 6]);
    assert_eq!(vec.capacity(), orig_capacity);
    assert_eq!(vec.as_ptr(), orig_ptr);

    // The split-off vector should be newly-allocated, and should not have
    // stolen the original vector's allocation.
    assert!(split_off.capacity() < orig_capacity);
    assert_ne!(split_off.as_ptr(), orig_ptr);
}

#[test]
fn test_into_iter_as_slice() {
    let vec = array_list!['a', 'b', 'c'].unwrap();
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    let _ = into_iter.next().unwrap();
    assert_eq!(into_iter.as_slice(), &['b', 'c']);
    let _ = into_iter.next().unwrap();
    let _ = into_iter.next().unwrap();
    assert_eq!(into_iter.as_slice(), &[]);
}

#[test]
fn test_into_iter_as_mut_slice() {
    let vec = array_list!['a', 'b', 'c'].unwrap();
    let mut into_iter = vec.into_iter();
    assert_eq!(into_iter.as_slice(), &['a', 'b', 'c']);
    into_iter.as_mut_slice()[0] = 'x';
    into_iter.as_mut_slice()[1] = 'y';
    assert_eq!(into_iter.next().unwrap(), 'x');
    assert_eq!(into_iter.as_slice(), &['y', 'c']);
}

#[test]
fn test_into_iter_debug() {
    let vec = array_list!['a', 'b', 'c'].unwrap();
    let into_iter = vec.into_iter();
    let debug = format!("{into_iter:?}");
    assert_eq!(debug, "IntoIter(['a', 'b', 'c'])");
}

#[test]
fn test_into_iter_count() {
    assert_eq!([1, 2, 3].into_iter().count(), 3);
}

#[test]
fn test_into_iter_clone() {
    fn iter_equal<I: Iterator<Item = i32>>(it: I, slice: &[i32]) {
        let v: ArrayList<i32> = it.collect();
        assert_eq!(&v[..], slice);
    }
    let mut it = [1, 2, 3].into_iter();
    iter_equal(it.clone(), &[1, 2, 3]);
    assert_eq!(it.next(), Some(1));
    let mut it = it.rev();
    iter_equal(it.clone(), &[3, 2]);
    assert_eq!(it.next(), Some(3));
    iter_equal(it.clone(), &[2]);
    assert_eq!(it.next(), Some(2));
    iter_equal(it.clone(), &[]);
    assert_eq!(it.next(), None);
}

#[test]
#[cfg_attr(not(panic = "unwind"), ignore = "test requires unwinding support")]
fn test_into_iter_leak() {
    static mut DROPS: i32 = 0;

    struct D(bool);

    impl Drop for D {
        fn drop(&mut self) {
            // Safety:
            unsafe {
                DROPS += 1;
            }

            if self.0 {
                panic!("panic in `drop`");
            }
        }
    }

    let v = array_list![D(false), D(true), D(false)].unwrap();

    catch_unwind(move || drop(v.into_iter())).ok();

    // Safety:
    assert_eq!(unsafe { DROPS }, 3);
}

#[test]
fn from_into_inner() {
    let vec = array_list![1, 2, 3].unwrap();
    let vec = vec.into_iter().collect::<ArrayList<_>>();
    assert_eq!(vec, [1, 2, 3]);

    let ptr = &vec[1] as *const _;
    let mut it = vec.into_iter();
    it.next().unwrap();
    let vec = it.collect::<ArrayList<_>>();
    assert_eq!(vec, [2, 3]);
    assert!(ptr != vec.as_ptr());
}

#[test]
fn overaligned_allocations() {
    #[repr(align(256))]
    struct Foo(usize);
    let mut v = array_list![Foo(273)].unwrap();
    for i in 0..0x1000 {
        v.reserve_exact(i).unwrap();
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
        v.shrink_to_fit().unwrap();
        assert!(v[0].0 == 273);
        assert!(v.as_ptr() as usize & 0xff == 0);
    }
}

#[test]
fn test_reserve_exact() {
    // This is all the same as test_reserve

    let mut v = ArrayList::new();
    assert_eq!(v.capacity(), 0);

    v.reserve_exact(2).unwrap();
    assert!(v.capacity() >= 2);

    for i in 0..16 {
        v.push(i).unwrap();
    }

    assert!(v.capacity() >= 16);
    v.reserve_exact(16).unwrap();
    assert!(v.capacity() >= 32);

    v.push(16).unwrap();

    v.reserve_exact(16).unwrap();
    assert!(v.capacity() >= 33);
}

#[test]
fn test_stable_pointers() {
    // Test that, if we reserved enough space, adding and removing elements does not
    // invalidate references into the vector (such as `v0`). This test also
    // runs in Miri, which would detect such problems.
    // Note that this test does *not* constitute a stable guarantee that all these functions do not
    // reallocate! Only what is explicitly documented at
    // <https://doc.rust-lang.org/nightly/std/vec/struct.ArrayList.html#guarantees> is stably guaranteed.
    let mut v = ArrayList::with_capacity(128).unwrap();
    v.push(13).unwrap();

    // Laundering the lifetime -- we take care that `v` does not reallocate, so that's okay.
    let v0 = &mut v[0];
    // Safety:
    let v0 = unsafe { &mut *(v0 as *mut _) };
    // Now do a bunch of things and occasionally use `v0` again to assert it is still valid.

    // Pushing/inserting and popping/removing
    v.push(1).unwrap();
    v.push(2).unwrap();
    v.insert(1, 1).unwrap();
    assert_eq!(*v0, 13);
    v.remove(1).unwrap();
    v.pop_back().unwrap();
    assert_eq!(*v0, 13);
    v.push(1).unwrap();
    v.swap_remove(1).unwrap();
    assert_eq!(v.len(), 2);
    v.swap_remove(1).unwrap(); // swap_remove the last element
    assert_eq!(*v0, 13);

    // Appending
    v.append(&mut array_list![27, 19].unwrap()).unwrap();
    assert_eq!(*v0, 13);

    // Extending
    v.extend(&[1, 2]); // `slice::Iter` (with `T: Copy`) specialization
    v.extend(array_list![2, 3].unwrap()); // `vec::IntoIter` specialization
    v.extend(std::iter::once(3)); // `TrustedLen` specialization
    v.extend(std::iter::empty::<i32>()); // `TrustedLen` specialization with empty iterator
    v.extend(std::iter::once(3).filter(|_| true)); // base case
    v.extend(std::iter::once(&3)); // `cloned` specialization
    assert_eq!(*v0, 13);

    // Truncation
    v.truncate(2);
    assert_eq!(*v0, 13);

    // Resizing
    v.resize_with(v.len() + 10, || 42).unwrap();
    assert_eq!(*v0, 13);
    v.resize_with(2, || panic!()).unwrap();
    assert_eq!(*v0, 13);

    // No-op reservation
    v.reserve(32).unwrap();
    v.reserve_exact(32).unwrap();
    assert_eq!(*v0, 13);

    // spare_capacity_mut
    v.spare_capacity_mut();
    assert_eq!(*v0, 13);

    // Smoke test that would fire even outside Miri if an actual relocation happened.
    // Also ensures the pointer is still writeable after all this.
    *v0 -= 13;
    assert_eq!(v[0], 0);
}

macro_rules! generate_assert_eq_vec_and_prim {
    ($name:ident<$B:ident>($type:ty)) => {
        fn $name<A: PartialEq<$B> + Debug, $B: Debug>(a: ArrayList<A>, b: $type) {
            assert!(a == b);
            assert_eq!(a, b);
        }
    };
}

generate_assert_eq_vec_and_prim! { assert_eq_vec_and_slice  <B>(&[B])   }
generate_assert_eq_vec_and_prim! { assert_eq_vec_and_array_3<B>([B; 3]) }

#[test]
fn partialeq_vec_and_prim() {
    assert_eq_vec_and_slice(array_list![1, 2, 3].unwrap(), &[1, 2, 3]);
    assert_eq_vec_and_array_3(array_list![1, 2, 3].unwrap(), [1, 2, 3]);
}

macro_rules! assert_partial_eq_valid {
    ($a2:expr, $a3:expr; $b2:expr, $b3: expr) => {
        assert!($a2 == $b2);
        assert!($a2 != $b3);
        assert!($a3 != $b2);
        assert!($a3 == $b3);
        assert_eq!($a2, $b2);
        assert_ne!($a2, $b3);
        assert_ne!($a3, $b2);
        assert_eq!($a3, $b3);
    };
}

#[test]
fn partialeq_vec_full() {
    let vec2: ArrayList<_> = array_list![1, 2].unwrap();
    let vec3: ArrayList<_> = array_list![1, 2, 3].unwrap();
    let slice2: &[_] = &[1, 2];
    let slice3: &[_] = &[1, 2, 3];
    let slicemut2: &[_] = &mut [1, 2];
    let slicemut3: &[_] = &mut [1, 2, 3];
    let array2: [_; 2] = [1, 2];
    let array3: [_; 3] = [1, 2, 3];
    let arrayref2: &[_; 2] = &[1, 2];
    let arrayref3: &[_; 3] = &[1, 2, 3];

    assert_partial_eq_valid!(vec2,vec3; vec2,vec3);
    assert_partial_eq_valid!(vec2,vec3; slice2,slice3);
    assert_partial_eq_valid!(vec2,vec3; slicemut2,slicemut3);
    assert_partial_eq_valid!(slice2,slice3; vec2,vec3);
    assert_partial_eq_valid!(slicemut2,slicemut3; vec2,vec3);
    assert_partial_eq_valid!(vec2,vec3; array2,array3);
    assert_partial_eq_valid!(vec2,vec3; arrayref2,arrayref3);
    assert_partial_eq_valid!(vec2,vec3; arrayref2[..],arrayref3[..]);
}

#[test]
fn test_zero_sized_vec_push() {
    const N: usize = 8;

    for len in 0..N {
        let mut tester = ArrayList::with_capacity(len).unwrap();
        assert_eq!(tester.len(), 0);
        assert!(tester.capacity() >= len);
        for _ in 0..len {
            tester.push(()).unwrap();
        }
        assert_eq!(tester.len(), len);
        assert_eq!(tester.iter().count(), len);
        tester.clear();
    }
}

#[test]
fn test_vec_macro_repeat() {
    assert_eq!(array_list![1; 3].unwrap(), array_list![1, 1, 1].unwrap());
    assert_eq!(array_list![1; 2].unwrap(), array_list![1, 1].unwrap());
    assert_eq!(array_list![1; 1].unwrap(), array_list![1].unwrap());
    assert_eq!(array_list![1; 0].unwrap(), array_list![].unwrap());

    // from_elem syntax (see RFC 832)
    let el = Box::new(1);
    let n = 3;
    assert_eq!(
        array_list![el; n].unwrap(),
        array_list![Box::new(1), Box::new(1), Box::new(1)].unwrap()
    );
}

#[test]
fn test_vec_swap() {
    let mut a: ArrayList<isize> = array_list![0, 1, 2, 3, 4, 5, 6].unwrap();
    a.swap(2, 4);
    assert_eq!(a[2], 4);
    assert_eq!(a[4], 2);
    let mut n = 42;
    swap(&mut n, &mut a[0]);
    assert_eq!(a[0], 42);
    assert_eq!(n, 0);
}

#[test]
fn test_extend_from_within_clone() {
    let mut v = vec![
        String::from("sssss"),
        String::from("12334567890"),
        String::from("c"),
    ];
    v.extend_from_within(1..);

    assert_eq!(v, ["sssss", "12334567890", "c", "12334567890", "c"]);
}

#[test]
fn test_vec_from_array_ref() {
    assert_eq!(ArrayList::from(&[1, 2, 3]), array_list![1, 2, 3].unwrap());
}

#[test]
fn test_vec_from_array_mut_ref() {
    assert_eq!(
        ArrayList::from(&mut [1, 2, 3]),
        array_list![1, 2, 3].unwrap()
    );
}
