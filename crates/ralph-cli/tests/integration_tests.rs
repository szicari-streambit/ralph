// ABOUTME: Integration tests for Ralph CLI commands
// ABOUTME: Tests init, status, and hook commands with temp directories

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn ralph_binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ralph"))
}

#[test]
fn test_ralph_help() {
    let output = ralph_binary().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Ralph CLI"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("plan"));
    assert!(stdout.contains("implement"));
    assert!(stdout.contains("status"));
    assert!(stdout.contains("hook"));
}

#[test]
fn test_ralph_version() {
    let output = ralph_binary().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ralph"));
}

#[test]
fn test_init_creates_structure() {
    let temp = TempDir::new().unwrap();

    let output = ralph_binary()
        .arg("init")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    // Check directories were created
    assert!(temp.path().join("ralph/tasks").exists());
    assert!(temp.path().join("docs/ralph").exists());
    assert!(temp.path().join(".github/agents").exists());
    assert!(temp.path().join(".githooks").exists());

    // Check files were created
    assert!(temp
        .path()
        .join(".github/agents/ralph-planner.agent.md")
        .exists());
    assert!(temp
        .path()
        .join(".github/agents/ralph-implementer.agent.md")
        .exists());
    assert!(temp.path().join(".githooks/commit-msg").exists());
    assert!(temp.path().join("ralph/validation.json").exists());
}

#[test]
fn test_init_dry_run() {
    let temp = TempDir::new().unwrap();

    let output = ralph_binary()
        .args(["init", "--dry-run"])
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    // Dry run should not create anything
    assert!(!temp.path().join("ralph/tasks").exists());
    assert!(!temp.path().join(".github/agents").exists());
}

#[test]
fn test_status_no_features() {
    let temp = TempDir::new().unwrap();

    // Initialize first
    ralph_binary()
        .arg("init")
        .current_dir(temp.path())
        .output()
        .unwrap();

    let output = ralph_binary()
        .arg("status")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No features found"));
}

#[test]
fn test_status_with_feature() {
    let temp = TempDir::new().unwrap();

    // Initialize
    ralph_binary()
        .arg("init")
        .current_dir(temp.path())
        .output()
        .unwrap();

    // Create a feature manually
    let task_dir = temp.path().join("ralph/tasks/test-feature");
    fs::create_dir_all(&task_dir).unwrap();

    let prd = r#"{
        "schemaVersion": "1.0",
        "slug": "test-feature",
        "title": "Test Feature",
        "activeRunId": "test-20260119",
        "validationProfiles": ["rust-cargo"],
        "requirements": [
            {
                "id": "REQ-01",
                "title": "First requirement",
                "status": "done",
                "acceptanceCriteria": ["Test criterion"]
            }
        ]
    }"#;
    fs::write(task_dir.join("prd.json"), prd).unwrap();

    let output = ralph_binary()
        .arg("status")
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Test Feature"));
}

#[test]
fn test_hook_commit_msg_valid() {
    let temp = TempDir::new().unwrap();

    // Create commit message file
    let msg_file = temp.path().join("commit-msg.txt");
    fs::write(&msg_file, "REQ-01: Add feature").unwrap();

    let output = ralph_binary()
        .args(["hook", "commit-msg", msg_file.to_str().unwrap()])
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());
}

#[test]
fn test_hook_commit_msg_invalid() {
    let temp = TempDir::new().unwrap();

    // Create commit message file without requirement reference
    let msg_file = temp.path().join("commit-msg.txt");
    fs::write(&msg_file, "Add feature without reference").unwrap();

    let output = ralph_binary()
        .args(["hook", "commit-msg", msg_file.to_str().unwrap()])
        .current_dir(temp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("must reference a requirement"));
}
