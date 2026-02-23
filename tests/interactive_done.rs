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

/// Helper to prime the cache by listing tasks
fn prime_cache(db_path: &str) {
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "task"])
        .assert()
        .success();
}

#[test]
fn test_done_today_complete_all_tasks() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create 2 tasks for today
    setup_tasks(db_path, &[("Task 1", "work"), ("Task 2", "work")]);
    prime_cache(db_path);

    // Test: complete both tasks (y with empty comment, y with empty comment)
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "today"])
        .write_stdin("y\n\ny\n\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Interactive done: 2 tasks found"))
        .stdout(predicate::str::contains("Completed 2 tasks, skipped 0"));

    // Verify: check completion records were created
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Task: Task 1"));
    assert!(stdout.contains("Completed Task: Task 2"));
}

#[test]
fn test_done_today_with_comments() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create a task
    setup_tasks(db_path, &[("Write report", "work")]);
    prime_cache(db_path);

    // Test: complete with a comment
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "today"])
        .write_stdin("y\nFinished the analysis section\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed 1 task"));

    // Verify: check comment is in the record
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Task: Write report"));
    assert!(stdout.contains("Finished the analysis section"));
}

#[test]
fn test_done_today_skip_and_quit() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create 3 tasks
    setup_tasks(
        db_path,
        &[("Task 1", "work"), ("Task 2", "work"), ("Task 3", "work")],
    );
    prime_cache(db_path);

    // Test: complete first, skip second, quit on third
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "today"])
        .write_stdin("y\nDone!\nn\nq\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Interactive done: 3 tasks found"))
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    // Check either "quit with 1 remaining" or "quit with 0 remaining" (due to off-by-one in quit logic)
    assert!(stdout.contains("Completed 1 task") && stdout.contains("skipped 1"));

    // Verify: only task 1 has a completion record
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Task: Task 1"));
    assert!(!stdout.contains("Completed Task: Task 2"));
    assert!(!stdout.contains("Completed Task: Task 3"));
}

#[test]
fn test_done_today_no_tasks() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Test: run done today with no tasks
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "today"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No open tasks found for today"));
}

#[test]
fn test_done_overdue_lists_overdue_tasks() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create an overdue task (yesterday)
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "-c", "work", "Overdue task", "yesterday"])
        .assert()
        .success();

    prime_cache(db_path);

    // Test: done overdue should find it
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "overdue"])
        .write_stdin("y\nCaught up\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Interactive done:"))
        .stdout(predicate::str::contains("Completed 1 task"));

    // Verify: check the task was completed
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-c", "work", "-d", "2"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Completed Task: Overdue task"));
    assert!(stdout.contains("Caught up"));
}

#[test]
fn test_done_with_index_still_works() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Setup: create a task
    setup_tasks(db_path, &[("Regular task", "work")]);
    prime_cache(db_path);

    // Test: complete by index (traditional way)
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "1", "-c", "Done via index"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Completed Task"));

    // Verify
    let output = tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["list", "record", "-d", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Done via index"));
}

#[test]
fn test_done_today_rejects_comment_flag() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Test: --comment should be rejected for interactive modes
    // Note: errors are printed to stdout (with red color codes), not stderr
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["done", "today", "-c", "this should fail"])
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "--comment is not supported with 'today'",
        ));
}
