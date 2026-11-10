use headless_input_core::control::PointerAction;
use headless_input_core::player::{PointerOp, PointerOps};

#[test]
fn a_push_past_capacity_is_refused_and_keeps_the_first_ops() {
    let mut ops = PointerOps::new();
    for steps in 0..PointerOps::CAPACITY as i32 {
        assert!(ops.push(PointerOp::Scroll(steps)));
    }
    assert!(!ops.push(PointerOp::Press(PointerAction::Attack)));
    assert_eq!(ops.len(), PointerOps::CAPACITY);
    assert_eq!(
        ops.as_slice(),
        &[PointerOp::Scroll(0), PointerOp::Scroll(1), PointerOp::Scroll(2), PointerOp::Scroll(3)]
    );
}

#[test]
fn sets_with_the_same_ops_are_equal_after_a_clear() {
    let mut used = PointerOps::new();
    used.push(PointerOp::Press(PointerAction::Use));
    used.clear();
    used.push(PointerOp::Scroll(1));
    let mut fresh = PointerOps::new();
    fresh.push(PointerOp::Scroll(1));
    assert_eq!(used, fresh);
}
