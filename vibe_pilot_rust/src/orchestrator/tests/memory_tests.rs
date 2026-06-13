use crate::orchestrator::session_memory::{ActionStep, SessionMemory, MAX_HISTORY_STEPS};

#[test]
fn test_session_memory_new() {
    let mem = SessionMemory::new();
    assert_eq!(mem.total_actions_count, 0);
    assert_eq!(mem.successful_clicks, 0);
    assert_eq!(mem.scroll_count, 0);
    assert_eq!(mem.failed_attempts, 0);
    assert!(mem.steps.is_empty());
    assert!(mem.current_phase.is_empty());
    assert!(mem.last_significant_change.is_none());
}

#[test]
fn test_session_memory_record_click() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        coordinates: Some((0.5, 0.3)),
        text_typed: Some("hello".to_string()),
        llm_report: "Clicked on the search bar and typed hello to begin searching".to_string(),
        was_repeated: false,
    });
    assert_eq!(mem.total_actions_count, 1);
    assert_eq!(mem.successful_clicks, 1);
    assert_eq!(mem.scroll_count, 0);
    assert_eq!(mem.steps.len(), 1);
    assert!(!mem.current_phase.is_empty());
}

#[test]
fn test_session_memory_record_scroll() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:01".to_string(),
        action_type: "SCROLL".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Scrolling down to find the submit button".to_string(),
        was_repeated: false,
    });
    assert_eq!(mem.scroll_count, 1);
    assert_eq!(mem.successful_clicks, 0);
}

#[test]
fn test_session_memory_record_fail() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:02".to_string(),
        action_type: "FAIL".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Cannot find the button".to_string(),
        was_repeated: false,
    });
    assert_eq!(mem.failed_attempts, 1);
}

#[test]
fn test_session_memory_max_steps_limit() {
    let mut mem = SessionMemory::new();
    for i in 0..20 {
        mem.record(ActionStep {
            timestamp: format!("14:00:{:02}", i),
            action_type: "CLICK_AND_TYPE".to_string(),
            coordinates: Some((0.1 * i as f64, 0.2)),
            text_typed: None,
            llm_report: format!("Step {}", i),
            was_repeated: false,
        });
    }
    assert_eq!(mem.steps.len(), MAX_HISTORY_STEPS);
    assert_eq!(mem.total_actions_count, 20);
}

#[test]
fn test_session_memory_mark_significant_change() {
    let mut mem = SessionMemory::new();
    assert!(mem.last_significant_change.is_none());
    mem.mark_significant_change();
    assert!(mem.last_significant_change.is_some());
}

#[test]
fn test_session_memory_format_for_prompt_english() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        coordinates: Some((0.5, 0.3)),
        text_typed: Some("test".to_string()),
        llm_report: "Clicked on input field and typed test".to_string(),
        was_repeated: false,
    });
    mem.record(ActionStep {
        timestamp: "14:00:15".to_string(),
        action_type: "SCROLL".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Scrolled down".to_string(),
        was_repeated: false,
    });
    mem.mark_significant_change();

    let prompt = mem.format_for_prompt("English");
    assert!(prompt.contains("SESSION STATS"));
    assert!(prompt.contains("2 actions total"));
    assert!(prompt.contains("1 clicks"));
    assert!(prompt.contains("1 scrolls"));
    assert!(prompt.contains("RECENT ACTIONS"));
    assert!(prompt.contains("CLICK_AND_TYPE"));
    assert!(prompt.contains("SCROLL"));
}

#[test]
fn test_session_memory_format_for_prompt_french() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "FAIL".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Impossible de trouver le bouton".to_string(),
        was_repeated: true,
    });

    let prompt = mem.format_for_prompt("Français");
    assert!(prompt.contains("STATS SESSION"));
    assert!(prompt.contains("1 échecs"));
    assert!(prompt.contains("DERNIÈRES ACTIONS"));
    assert!(prompt.contains("[REPEATED]"));
}

#[test]
fn test_session_memory_format_empty() {
    let mem = SessionMemory::new();
    let prompt = mem.format_for_prompt("English");
    assert!(prompt.contains("0 actions total"));
    assert!(prompt.contains("RECENT ACTIONS"));
}

#[test]
fn test_session_memory_text_preview_truncation() {
    let mut mem = SessionMemory::new();
    let long_text = "a".repeat(50);
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        coordinates: Some((0.5, 0.5)),
        text_typed: Some(long_text),
        llm_report: "Typed a very long string into the form field".to_string(),
        was_repeated: false,
    });
    let prompt = mem.format_for_prompt("English");
    assert!(prompt.contains("..."));
}

#[test]
fn test_session_memory_phase_extraction() {
    let mut mem = SessionMemory::new();
    let long_report = "Navigating to the user settings page by clicking on the gear icon in the top right corner of the screen and waiting for the page to load completely";
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        coordinates: Some((0.9, 0.1)),
        text_typed: None,
        llm_report: long_report.to_string(),
        was_repeated: false,
    });
    // Phase should be truncated to ~80 chars
    assert!(mem.current_phase.len() <= 85);
    assert!(!mem.current_phase.is_empty());
}

#[test]
fn test_session_memory_stale_warning() {
    let mut mem = SessionMemory::new();
    // Manually set a last_significant_change that's old
    mem.last_significant_change = Some(std::time::Instant::now() - std::time::Duration::from_secs(60));
    let prompt = mem.format_for_prompt("English");
    assert!(prompt.contains("ALERT"));
    assert!(prompt.contains("No significant visual change"));
}

#[test]
fn test_session_memory_stale_warning_french() {
    let mut mem = SessionMemory::new();
    mem.last_significant_change = Some(std::time::Instant::now() - std::time::Duration::from_secs(60));
    let prompt = mem.format_for_prompt("Français");
    assert!(prompt.contains("ALERTE"));
    assert!(prompt.contains("Aucun changement visuel significatif"));
}

#[test]
fn test_session_memory_compressed_history() {
    let mut mem = SessionMemory::default();
    mem.compressed_history = "User navigated correctly.".to_string();
    let prompt_en = mem.format_for_prompt("English");
    assert!(prompt_en.contains("COMPRESSED HISTORY SUMMARY"));
    assert!(prompt_en.contains("User navigated correctly."));

    let prompt_fr = mem.format_for_prompt("Français");
    assert!(prompt_fr.contains("RÉSUMÉ DE L'HISTORIQUE COMPRESSÉ"));
    assert!(prompt_fr.contains("User navigated correctly."));
}

#[test]
fn test_session_memory_record_other_action_and_short_report() {
    let mut mem = SessionMemory::new();
    mem.record(ActionStep {
        timestamp: "14:00:00".to_string(),
        action_type: "WAIT".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Wait".to_string(), // <= 10 characters
        was_repeated: false,
    });
    assert_eq!(mem.total_actions_count, 1);
    assert_eq!(mem.successful_clicks, 0);
    assert_eq!(mem.scroll_count, 0);
    assert_eq!(mem.failed_attempts, 0);
    assert!(mem.current_phase.is_empty());
}

