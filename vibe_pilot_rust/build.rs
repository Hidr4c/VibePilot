//! Build script that injects git version info into the Rust binary.
//!
//! This script runs before compilation and sets environment variables
//! that are then accessible via `option_env!()` in the version module.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.git/refs/tags/");

    // Get current git branch
    let branch = get_git_branch();
    println!("cargo:warning=[vibepilot] Building from branch: {}", branch);

    // Get latest tag
    let tag = get_latest_tag();
    println!("cargo:warning=[vibepilot] Latest tag: {}", tag);

    // Get short commit hash
    let hash = get_git_hash();
    println!("cargo:warning=[vibepilot] Git hash: {}", hash);

    // Set environment variables for the Rust compiler
    println!("cargo:rustc-env=GIT_HASH={}", hash);
    println!("cargo:rustc-env=BUILD_DATE={}", get_build_date());

    // If we have a tag, use it as the version (override CARGO_PKG_VERSION)
    if !tag.is_empty() {
        println!("cargo:rustc-env=APP_VERSION={}", tag);
    } else {
        println!("cargo:rustc-env=APP_VERSION=dev-{}", hash);
    }
}

fn get_git_branch() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => String::from("unknown"),
    }
}

fn get_latest_tag() -> String {
    let output = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => String::from(""),
    }
}

fn get_git_hash() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => String::from("unknown"),
    }
}

fn get_build_date() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let days = secs / 86400;
    let time_rem = (secs % 86400) as u32;
    let h = time_rem / 3600;
    let m = (time_rem % 3600) / 60;
    let s = time_rem % 60;
    // Approximate date from Unix epoch
    let mut year: i64 = 1970;
    let mut month: i32 = 1;
    let mut remaining_days = days;
    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 { 366 } else { 365 };
        if remaining_days < days_in_year { break; }
        remaining_days -= days_in_year;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays = [0, 31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut md = 0i32;
    for i in 1i32..=12i32 {
        if remaining_days < mdays[i as usize] as i64 { month = i; md = (remaining_days + 1) as i32; break; }
        remaining_days -= mdays[i as usize] as i64;
    }
    if md == 0 { month = 12; md = 31; }
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", year, month, md, h, m, s)
}
