use crate::orchestrator::loop_breaker::{LoopBreaker, LoopAction};

#[test]
fn test_loop_breaker_new() {
    let lb = LoopBreaker::new();
    assert_eq!(lb.repetition_count(), 0);
    assert_eq!(lb.escalation_level, 0);
    assert_eq!(lb.last_action_key, "");
}

#[test]
fn test_loop_breaker_no_repeat() {
    let mut lb = LoopBreaker::new();
    let result = lb.register_action("action_a");
    assert_eq!(result, LoopAction::Normal);
    assert_eq!(lb.repetition_count(), 0);

    let result = lb.register_action("action_b");
    assert_eq!(result, LoopAction::Normal);
    assert_eq!(lb.repetition_count(), 0);
}

#[test]
fn test_loop_breaker_escalation_level_1_warn() {
    let mut lb = LoopBreaker::new();
    let result1 = lb.register_action("action_a"); // first time: Normal
    assert_eq!(result1, LoopAction::Normal);
    let result2 = lb.register_action("action_a"); // 2nd time: repetition_count = 1, level 1 -> WarnLlm
    assert_eq!(result2, LoopAction::WarnLlm);
    assert_eq!(lb.repetition_count(), 1);
}

#[test]
fn test_loop_breaker_escalation_level_2_desktop() {
    let mut lb = LoopBreaker::new();
    for _ in 0..4 {
        lb.register_action("a");
    }
    let result = lb.register_action("a"); // 5th = repetition_count 4, level 2 -> EscalateToDesktop
    assert_eq!(result, LoopAction::EscalateToDesktop);
}

#[test]
fn test_loop_breaker_escalation_level_3_escape_keys() {
    let mut lb = LoopBreaker::new();
    for _ in 0..5 {
        lb.register_action("a");
    }
    let result = lb.register_action("a"); // 6th = repetition_count 5, level 3 -> SendEscapeKeys
    assert_eq!(result, LoopAction::SendEscapeKeys);
}

#[test]
fn test_loop_breaker_escalation_level_5_alert() {
    let mut lb = LoopBreaker::new();
    for _ in 0..7 {
        lb.register_action("a");
    }
    let result = lb.register_action("a"); // 9th = repetition_count 8, level 5 -> AlertUser
    assert_eq!(result, LoopAction::AlertUser);
}

#[test]
fn test_loop_breaker_reset_on_different_action() {
    let mut lb = LoopBreaker::new();
    lb.register_action("a");
    lb.register_action("a");
    assert_eq!(lb.repetition_count(), 1);

    lb.register_action("b"); // different action resets
    assert_eq!(lb.repetition_count(), 0);
    assert_eq!(lb.escalation_level, 0);
}

#[test]
fn test_loop_breaker_manual_reset() {
    let mut lb = LoopBreaker::new();
    lb.register_action("a");
    lb.register_action("a");
    lb.reset();
    assert_eq!(lb.repetition_count(), 0);
    assert_eq!(lb.escalation_level, 0);
    assert_eq!(lb.total_consecutive_failures, 0);
}

#[test]
fn test_loop_action_debug() {
    assert_eq!(format!("{:?}", LoopAction::Normal), "Normal");
    assert_eq!(format!("{:?}", LoopAction::WarnLlm), "WarnLlm");
    assert_eq!(format!("{:?}", LoopAction::EscalateToDesktop), "EscalateToDesktop");
    assert_eq!(format!("{:?}", LoopAction::SendEscapeKeys), "SendEscapeKeys");
    assert_eq!(format!("{:?}", LoopAction::ForceScroll), "ForceScroll");
    assert_eq!(format!("{:?}", LoopAction::AlertUser), "AlertUser");
}

#[test]
fn test_loop_action_equality() {
    assert_eq!(LoopAction::Normal, LoopAction::Normal);
    assert_ne!(LoopAction::Normal, LoopAction::WarnLlm);
    assert_ne!(LoopAction::EscalateToDesktop, LoopAction::AlertUser);
}

#[test]
fn test_loop_breaker_full_escalation_sequence() {
    let mut lb = LoopBreaker::new();
    let results: Vec<LoopAction> = (0..9)
        .map(|_| lb.register_action("same_action"))
        .collect();
    
    assert_eq!(results[0], LoopAction::Normal);      // 1st time (rep=0)
    assert_eq!(results[1], LoopAction::WarnLlm);     // 2nd (rep=1)
    assert_eq!(results[2], LoopAction::WarnLlm);     // 3rd (rep=2)
    assert_eq!(results[3], LoopAction::WarnLlm);     // 4th (rep=3)
    assert_eq!(results[4], LoopAction::EscalateToDesktop); // 5th (rep=4)
    assert_eq!(results[5], LoopAction::SendEscapeKeys);    // 6th (rep=5)
    assert_eq!(results[6], LoopAction::ForceScroll);       // 7th (rep=6)
    assert_eq!(results[7], LoopAction::AlertUser);         // 8th (rep=7)
    assert_eq!(results[8], LoopAction::AlertUser);         // 9th stays at AlertUser
}

#[test]
fn test_loop_breaker_precise_coordinates() {
    let mut lb = LoopBreaker::new();
    
    // 1. First execution
    let res = lb.register_action_precise(
        "CLICK_AND_TYPE:0.15:0.20:0",
        "CLICK_AND_TYPE",
        Some((0.150, 0.200)),
        "",
        0,
        &[],
    );
    assert_eq!(res, LoopAction::Normal);
    assert_eq!(lb.repetition_count(), 0);

    // 2. Second execution close to the first (distance ~0.0054 < 0.03)
    let res2 = lb.register_action_precise(
        "CLICK_AND_TYPE:0.15:0.20:0",
        "CLICK_AND_TYPE",
        Some((0.155, 0.202)),
        "",
        0,
        &[],
    );
    assert_eq!(res2, LoopAction::WarnLlm);
    assert_eq!(lb.repetition_count(), 1);

    // 3. Third execution far from the second (distance 0.04 > 0.03) -> resets repetition
    let res3 = lb.register_action_precise(
        "CLICK_AND_TYPE:0.19:0.20:0",
        "CLICK_AND_TYPE",
        Some((0.190, 0.200)),
        "",
        0,
        &[],
    );
    assert_eq!(res3, LoopAction::Normal);
    assert_eq!(lb.repetition_count(), 0);
}

#[test]
fn test_loop_breaker_precise_mismatch_combinations() {
    let mut lb = LoopBreaker::default();
    
    // Initial action with coords and text and keys
    lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        10,
        &["KeyA".to_string()],
    );

    // 1. Coords mismatch: Some & None
    let res = lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        None,
        "hello",
        10,
        &["KeyA".to_string()],
    );
    assert_eq!(res, LoopAction::Normal);

    // Reset loop state to original action
    lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        10,
        &["KeyA".to_string()],
    );

    // 2. Coords match, but text mismatch
    let res = lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "different",
        10,
        &["KeyA".to_string()],
    );
    assert_eq!(res, LoopAction::Normal);

    // Reset loop state to original action
    lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        10,
        &["KeyA".to_string()],
    );

    // 3. Coords & text match, but scroll mismatch
    let res = lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        20,
        &["KeyA".to_string()],
    );
    assert_eq!(res, LoopAction::Normal);

    // Reset loop state to original action
    lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        10,
        &["KeyA".to_string()],
    );

    // 4. Coords & text & scroll match, but keys mismatch
    let res = lb.register_action_precise(
        "k",
        "CLICK_AND_TYPE",
        Some((0.1, 0.2)),
        "hello",
        10,
        &["KeyB".to_string()],
    );
    assert_eq!(res, LoopAction::Normal);

    // 5. None & None coords match
    lb.register_action_precise(
        "k",
        "KEY_COMBO",
        None,
        "",
        0,
        &["KeyA".to_string()],
    );
    let res_none_none = lb.register_action_precise(
        "k",
        "KEY_COMBO",
        None,
        "",
        0,
        &["KeyA".to_string()],
    );
    assert_eq!(res_none_none, LoopAction::WarnLlm);
}


