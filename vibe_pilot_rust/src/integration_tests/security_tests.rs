#[test]
fn test_dangerous_keywords_detection() {
    let dangerous_keywords = [
        "format", "shutdown", "sudo", "rm -rf", "drop table",
        "delete from", "chmod", "wget http", "curl http",
        "pip install", "npm install", "taskkill", "reg delete",
        "kill -9", "desactiver antivirus", "turn off firewall",
        "rmdir", "apt remove", "apt purge", "bcdedit", "dd if=",
    ];

    for keyword in &dangerous_keywords {
        assert!(crate::config::contains_dangerous_command(keyword), "Keyword '{}' should be flagged as dangerous", keyword);
        assert!(crate::config::contains_dangerous_command(&format!("some prefix {} suffix", keyword)));
    }
}

#[test]
fn test_safe_text_not_flagged() {
    let safe_texts = [
        "Hello world",
        "Normal message",
        "Standard operation",
        "Regular computation",
    ];

    for text in &safe_texts {
        assert!(!crate::config::contains_dangerous_command(text), "Text '{}' should not be flagged as dangerous", text);
    }
}

#[test]
fn test_empty_text_not_flagged() {
    assert!(!crate::config::contains_dangerous_command(""));
}

#[test]
fn test_is_dangerous_shortcut_detects_dangerous() {
    let dangerous = ["close", "close_tab", "show_desktop", "minimize", "task_view", "force_quit", "kill_process", "shutdown"];
    for shortcut in &dangerous {
        assert!(crate::config::is_dangerous_shortcut(shortcut), "Shortcut '{}' should be dangerous", shortcut);
        assert!(crate::config::is_dangerous_shortcut(&shortcut.to_uppercase()), "Shortcut '{}' (uppercase) should be dangerous", shortcut);
    }
}

#[test]
fn test_is_dangerous_shortcut_allows_safe() {
    let safe = ["copy", "paste", "undo", "redo", "save", "new_tab", "find", "select_all"];
    for shortcut in &safe {
        assert!(!crate::config::is_dangerous_shortcut(shortcut), "Shortcut '{}' should not be dangerous", shortcut);
    }
}

#[test]
fn test_is_dangerous_shortcut_empty() {
    assert!(!crate::config::is_dangerous_shortcut(""));
}

#[test]
fn test_is_dangerous_shortcut_partial_match_not_flagged() {
    assert!(!crate::config::is_dangerous_shortcut("close_window_but_keep_process"));
    assert!(!crate::config::is_dangerous_shortcut("not_show_desktop"));
}

#[test]
fn test_export_decrypted_journal_file_not_found() {
    let result = crate::config::export_decrypted_journal(
        std::path::Path::new("nonexistent_file.enc"),
        std::path::Path::new("output.json"),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Failed to read"));
}
