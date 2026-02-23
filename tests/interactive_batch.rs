use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

/// Helper to get a clean test database path
fn get_test_db() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp file")
}

/// Helper to run tascli command with test database
fn tascli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tascli"))
}

/// Helper to setup tasks for testing
fn setup_tasks(db_path: &str, tasks: &[(&str, &str)]) {
    for (content, category) in tasks {
        tascli()
            .env("TASCLI_TEST_DB", db_path)
            .args(&["task", "-c", category, content, "today"])
            .assert()
            .success();
    }
}

/// Helper to setup records for testing
fn setup_records(db_path: &str, records: &[(&str, &str)]) {
    for (content, category) in records {
        tascli()
            .env("TASCLI_TEST_DB", db_path)
            .args(&["record", "-c", category, content])
            .assert()
            .success();
    }
}

#[test]
fn test_batch_interactive_update_category_all_yes() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create tasks in "old" category
    setup_tasks(db_path, &[("Task 1", "old"), ("Task 2", "old")]);

    // Test: interactively update category, accept all
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "-c",
            "old",
            "--category-to",
            "new",
            "--interactive",
        ])
        .write_stdin("y\ny\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Item 1/2"))
        .stdout(predicate::str::contains("Item 2/2"))
        .stdout(predicate::str::contains("Updated 2 items, skipped 0"));

    // Verify: tasks now in "new" category
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "new"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Task 1"));
    assert!(stdout.contains("Task 2"));
}

#[test]
fn test_batch_interactive_delete_with_skip_and_quit() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create 3 records
    setup_records(
        db_path,
        &[
            ("Record 1", "test"),
            ("Record 2", "test"),
            ("Record 3", "test"),
        ],
    );

    // Test: interactive delete - yes, no, quit
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["ops", "batch", "-c", "test", "--delete", "--interactive"])
        .write_stdin("y\nn\nq\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Item 1/3"))
        .stdout(predicate::str::contains("Deleted 1 item, skipped 1, quit with 1 remaining"));

    // Verify: only record 1 was deleted, records 2 and 3 remain
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "test", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(!stdout.contains("Record 1")); // Deleted
    assert!(stdout.contains("Record 2")); // Kept
    assert!(stdout.contains("Record 3")); // Kept
}

#[test]
fn test_batch_interactive_update_task_status() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create pending tasks
    setup_tasks(
        db_path,
        &[("Task 1", "work"), ("Task 2", "work"), ("Task 3", "work")],
    );

    // Test: interactively mark as cancelled (status 2) - yes, yes, no
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "--action",
            "task",
            "-c",
            "work",
            "--status-to",
            "cancelled",
            "--interactive",
        ])
        .write_stdin("y\ny\nn\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 2 items, skipped 1"));

    // Verify: first 2 tasks cancelled, third still open
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "work", "--status", "cancelled"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Task 1"));
    assert!(stdout.contains("Task 2"));
    assert!(!stdout.contains("Task 3")); // Still open, not cancelled
}

#[test]
fn test_batch_interactive_no_items() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Test: batch with no matching items
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "-c",
            "nonexistent",
            "--category-to",
            "new",
            "--interactive",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("No items found matching the filters"));
}

#[test]
fn test_batch_interactive_with_time_filters() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create tasks for today and tomorrow
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Today task", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Tomorrow task", "tomorrow"])
        .assert()
        .success();

    // Test: interactive batch only on today's tasks
    // Note: filter might find multiple items, provide enough responses
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "--action",
            "task",
            "-c",
            "work",
            "-e",
            "today",
            "--category-to",
            "urgent",
            "--interactive",
        ])
        .write_stdin("y\ny\n")  // Provide responses for potentially 2 items
        .assert()
        .success();

    // Verify: at least today's task was updated to urgent category
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "urgent"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Today task"));
}

#[test]
fn test_batch_interactive_with_status_filter() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create mix of open and completed tasks
    setup_tasks(db_path, &[("Open task", "work"), ("Another open", "work")]);

    // Complete one task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "work"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "1"])
        .assert()
        .success();

    // Test: batch only on open tasks with status filter
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "--action",
            "task",
            "-c",
            "work",
            "--status",
            "open",
            "--category-to",
            "pending",
            "--interactive",
        ])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Item 1/1"));

    // Verify: only the open task was updated
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "pending"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Another open"));
    assert!(!stdout.contains("Open task")); // This was completed, not in pending
}

#[test]
fn test_batch_non_interactive_still_works() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create tasks
    setup_tasks(db_path, &[("Task 1", "old"), ("Task 2", "old")]);

    // Test: non-interactive batch (single confirmation prompt, not per-item)
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["ops", "batch", "-c", "old", "--category-to", "new"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Proceed? (y/n):"))
        .stdout(predicate::str::contains("Successfully updated 2 items"));

    // Verify: all tasks updated
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "new"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Task 1"));
    assert!(stdout.contains("Task 2"));
}

#[test]
fn test_batch_interactive_mixed_actions() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create both tasks and records in same category
    setup_tasks(db_path, &[("Task item", "mixed")]);
    setup_records(db_path, &[("Record item", "mixed")]);

    // Test: batch on "all" action type updates both
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&[
            "ops",
            "batch",
            "--action",
            "all",
            "-c",
            "mixed",
            "--category-to",
            "updated",
            "--interactive",
        ])
        .write_stdin("y\ny\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated 2 items"));

    // Verify: both task and record updated
    let task_output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "updated"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let record_output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "updated", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert!(String::from_utf8_lossy(&task_output).contains("Task item"));
    assert!(String::from_utf8_lossy(&record_output).contains("Record item"));
}
