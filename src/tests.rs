use crate::ArrayVec;

fn init_stack_full() -> ArrayVec<i32, 5> {
    let mut nums = ArrayVec::new();
    (1..=5).for_each(|i| nums.push(i));
    nums
}

fn init_stack_half_full() -> ArrayVec<i32, 5> {
    let mut nums = ArrayVec::new();
    (1..=3).for_each(|i| nums.push(i));
    nums
}

#[test]
fn pushing_nums() {
    let nums = init_stack_full();
    assert_eq!(nums.into_array().unwrap(), [1, 2, 3, 4, 5]);
}

#[test]
fn popping_nums() {
    let mut nums = init_stack_full();
    assert_eq!(nums.pop(), Some(5));
    assert_eq!(nums.pop(), Some(4));
    assert_eq!(nums.pop(), Some(3));
    assert_eq!(nums.pop(), Some(2));
    assert_eq!(nums.pop(), Some(1));
    assert_eq!(nums.pop(), None);
}

#[test]
fn remove_nums() {
    let mut nums = init_stack_full();
    assert_eq!(nums.remove(1), 2);
    assert_eq!(*nums, [1, 3, 4, 5]);
    let mut nums = init_stack_half_full();
    nums.remove(1);
    assert_eq!(*nums, [1, 3]);
    nums.extend(4..7);
    assert_eq!(*nums, [1, 3, 4, 5, 6]);
    assert!(nums.is_full());
    (0..nums.capacity()).for_each(|_| {
        nums.pop();
    });
    assert!(nums.is_empty());
}

#[test]
fn insert_nums() {
    let mut nums = init_stack_half_full();
    nums.insert(1, 14);
    assert_eq!(*nums, [1, 14, 2, 3]);
}

#[test]
fn retain_nums() {
    let mut nums = ArrayVec::<_, 10>::new();
    nums.extend(0..10);
    nums.retain(|i| i % 2 == 0);
    assert_eq!(*nums, [0, 2, 4, 6, 8]);
}

#[test]
fn len_nums() {
    let nums = init_stack_half_full();
    assert_eq!(nums.len(), 3);
    assert_eq!(nums.capacity(), 5);
}

#[test]
fn get_nums() {
    let mut nums = init_stack_half_full();
    assert_eq!(nums.get(0), Some(&1));
    assert_eq!(nums.get_mut(1), Some(&mut 2));
    assert_eq!(nums.get(3), None);
    assert_eq!(nums.get_mut(4), None);
    assert_eq!(nums.get(122), None);
}

#[test]
fn slicing_full() {
    let mut nums = init_stack_full();
    let mut array = [1, 2, 3, 4, 5];
    assert_eq!(nums.as_slice(), &array);
    assert_eq!(nums.as_mut_slice(), &mut array);
}

#[test]
fn slicing_half_full() {
    let mut nums = init_stack_half_full();
    let mut array = [1, 2, 3];
    assert_eq!(nums.as_slice(), &array);
    assert_eq!(nums.as_mut_slice(), &mut array);
}

struct TestDrop<'a> {
    has_dropped: &'a mut bool,
}

impl<'a> Drop for TestDrop<'a> {
    fn drop(&mut self) {
        assert!(!*self.has_dropped, "double free");
        *self.has_dropped = true;
    }
}

#[test]
fn drop_working() {
    let mut has_dropped = false;
    let test_drop = TestDrop {
        has_dropped: &mut has_dropped,
    };
    let mut array = ArrayVec::<_, 10>::new();
    array.push(test_drop);
    let test_drop = array.pop().unwrap();
    drop(test_drop);
    drop(array);
    assert!(has_dropped);
}
