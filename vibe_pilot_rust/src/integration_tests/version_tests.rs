use crate::version::{get_version, get_git_hash, get_build_date, get_version_info};

#[test]
fn test_get_version() {
    let version = get_version();
    assert!(!version.is_empty());
}

#[test]
fn test_get_git_hash() {
    let hash = get_git_hash();
    assert!(!hash.is_empty());
}

#[test]
fn test_get_build_date() {
    let date = get_build_date();
    assert!(!date.is_empty());
    assert!(date.contains("-"));
}

#[test]
fn test_get_version_info() {
    let info = get_version_info();
    assert!(info.contains("VibePilot"));
    assert!(info.contains("v"));
}
