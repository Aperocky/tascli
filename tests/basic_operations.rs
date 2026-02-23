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

#[test]
fn test_task_create_and_list() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create a task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Complete project", "tomorrow"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Inserted Task"))
        .stdout(predicate::str::contains("Complete project"));

    // List tasks
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "work"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Complete project"));
    assert!(stdout.contains("work"));
}

#[test]
fn test_record_create_and_list() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create a record
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["record", "-c", "fitness", "Ran 5km"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Inserted Record"))
        .stdout(predicate::str::contains("Ran 5km"));

    // List records
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "fitness", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Ran 5km"));
    assert!(stdout.contains("fitness"));
}

#[test]
fn test_task_done_creates_record() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create and list task to populate cache
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Write tests", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Complete the task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed Task"));

    // Verify record was created
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Task: Write tests"));
}

#[test]
fn test_task_update() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create and list task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "home", "Original task", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Update the task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["update", "1", "-w", "Updated task content"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated Task"))
        .stdout(predicate::str::contains("Updated task content"));

    // Verify update persisted
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "home"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Updated task content"));
    assert!(!stdout.contains("Original task"));
}

#[test]
fn test_task_delete() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create two tasks
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "test", "Task 1", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "test", "Task 2", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Delete first task (with confirmation)
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["delete", "1"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Deletion success"));

    // Verify only task 2 remains
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "test"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(!stdout.contains("Task 1"));
    assert!(stdout.contains("Task 2"));
}

#[test]
fn test_recurring_task_workflow() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create recurring task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "routine", "Daily exercise", "Daily 6AM"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Inserted Recurring Task"))
        .stdout(predicate::str::contains("Daily exercise"));

    // List and verify it shows as recurring
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Daily exercise"));
    assert!(stdout.contains("Recurring") || stdout.contains("Daily"));

    // Complete the recurring task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed Recurring Task"));

    // Verify recurring record was created
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "routine", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Recurring Task: Daily exercise"));
}

#[test]
fn test_list_with_status_filter() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create tasks with different statuses
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "test", "Open task", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "test", "To be completed", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Complete one task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "2"])
        .assert()
        .success();

    // List only open tasks
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "test", "--status", "open"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Open task"));
    assert!(!stdout.contains("To be completed"));

    // List only done tasks
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task", "-c", "test", "--status", "done"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(!stdout.contains("Open task"));
    assert!(stdout.contains("To be completed"));
}

#[test]
fn test_record_with_time_filters() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create records
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["record", "-c", "log", "Event 1"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["record", "-c", "log", "Event 2"])
        .assert()
        .success();

    // List records from today
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "log", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Event 1"));
    assert!(stdout.contains("Event 2"));
}

#[test]
fn test_task_with_comment() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Task with notes", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Complete with comment
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "1", "-c", "Finished early!"])
        .assert()
        .success();

    // Verify comment is in record
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Finished early!"));
}

#[test]
fn test_show_content_command() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Create task with multiline content
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Task with details", "today"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Add content to the task
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["update", "1", "-a", "Additional details here"])
        .assert()
        .success();

    // Show the content
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "show", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Task with details"));
    assert!(stdout.contains("Additional details here"));
}
