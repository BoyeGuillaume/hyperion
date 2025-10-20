use hyformal::utils::staticvec::StaticVec;

#[test]
fn basic_push_pop_len() {
    let mut v: StaticVec<i32, 4> = StaticVec::new();
    assert!(v.is_empty());
    v.push(1);
    v.push(2);
    v.push(3);
    assert_eq!(v.len(), 3);
    assert_eq!(v.as_slice(), &[1, 2, 3]);
    assert_eq!(v.pop(), Some(3));
    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), Some(1));
    assert_eq!(v.pop(), None);
}

#[test]
fn insert_and_erase() {
    let mut v: StaticVec<&str, 5> = StaticVec::new();
    v.push("a");
    v.push("c");
    v.insert(1, "b");
    assert_eq!(v.as_slice(), &["a", "b", "c"]);
    v.erase(1);
    assert_eq!(v.as_slice(), &["a", "c"]);
}

#[test]
fn iter_and_collect() {
    let mut v: StaticVec<u8, 3> = StaticVec::new();
    v.push(5);
    v.push(6);
    v.push(7);
    let collected: Vec<_> = v.iter().cloned().collect();
    assert_eq!(collected, vec![5, 6, 7]);
}

#[test]
#[should_panic]
fn overflow_panics() {
    let mut v: StaticVec<u8, 1> = StaticVec::new();
    v.push(1);
    // This push should panic due to capacity 1
    v.push(2);
}
