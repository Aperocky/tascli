use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn get_test_db() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp file")
}

fn tascli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tascli"))
}

#[test]
fn test_merge_nonexistent_source() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["ops", "merge", "--path", "/nonexistent/db.sqlite"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("not found"));
}

#[test]
fn test_merge_empty_source_into_populated_dest() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    // Populate dest with a task
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["task", "-c", "work", "existing task", "tomorrow"])
        .assert()
        .success();

    // Initialize source db by running any command (creates schema)
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Merge empty source
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 items inserted"))
        .stdout(predicate::str::contains("0 duplicates skipped"));

    // Dest still has its original task
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("existing task"));
}

#[test]
fn test_merge_into_empty_dest() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    // Populate source with tasks and records
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "-c", "work", "source task", "tomorrow"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["record", "-c", "log", "source record"])
        .assert()
        .success();

    // Initialize dest
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Merge
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 items inserted"))
        .stdout(predicate::str::contains("0 duplicates skipped"));

    // Verify both items present in dest
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source task"));

    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "record", "-d", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source record"));
}

#[test]
fn test_merge_with_duplicates_skipped() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    // Create items in source first
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "-c", "work", "shared task", "2099/01/01"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "-c", "personal", "unique task", "2099/02/01"])
        .assert()
        .success();

    // Copy source to dest so "shared task" is byte-identical (same create_time)
    std::fs::copy(source_path, dest_path).unwrap();

    // Remove "unique task" from dest by deleting it, so only "shared task" remains
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success();
    // Index 2 is "unique task" (sorted by target_time, 2099/01/01 < 2099/02/01)
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["delete", "2"])
        .write_stdin("y\n")
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 items inserted"))
        .stdout(predicate::str::contains("1 duplicates skipped"));

    // Verify unique task was merged back
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("unique task"));
}

#[test]
fn test_merge_tasks_and_records_separately() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    // Dest has only tasks
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["task", "-c", "work", "dest task", "2099/06/01"])
        .assert()
        .success();

    // Source has only records
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["record", "-c", "log", "source record 1"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["record", "-c", "log", "source record 2"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 items inserted"));

    // Dest now has both tasks and records
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("dest task"));

    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "record", "-d", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("source record 1"))
        .stdout(predicate::str::contains("source record 2"));
}

#[test]
fn test_merge_creates_backup() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    // Initialize both
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["task", "something", "tomorrow"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "other thing", "tomorrow"])
        .assert()
        .success();

    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("backing up"))
        .stdout(predicate::str::contains("Backed up to"));
}

#[test]
fn test_merge_idempotent() {
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();

    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "-c", "work", "task A", "2099/01/01"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["record", "-c", "log", "record B"])
        .assert()
        .success();

    // Initialize dest
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // First merge
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("2 items inserted"));

    // Second merge — should be all duplicates
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success()
        .stdout(predicate::str::contains("0 items inserted"))
        .stdout(predicate::str::contains("2 duplicates skipped"));
}

#[test]
fn test_merge_self_rejected() {
    let db = get_test_db();
    let db_path = db.path().to_str().unwrap();

    // Initialize db
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["task", "something", "tomorrow"])
        .assert()
        .success();

    // Merge into itself should fail
    tascli()
        .env("TASCLI_TEST_DB", db_path)
        .args(&["ops", "merge", "--path", db_path])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Source and destination are the same database"));
}

#[test]
fn test_merge_preserves_completed_task_status() {
    let source_db = get_test_db();
    let source_path = source_db.path().to_str().unwrap();
    let dest_db = get_test_db();
    let dest_path = dest_db.path().to_str().unwrap();

    // Create and complete a task in source
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["task", "-c", "work", "finish report", "2099/01/01"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["list", "task"])
        .assert()
        .success();
    tascli()
        .env("TASCLI_TEST_DB", source_path)
        .args(&["done", "1"])
        .assert()
        .success();

    // Initialize dest
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task"])
        .assert()
        .success();

    // Merge
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["ops", "merge", "--path", source_path])
        .assert()
        .success();

    // The completed task should show up under done status
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "task", "-s", "done"])
        .assert()
        .success()
        .stdout(predicate::str::contains("finish report"))
        .stdout(predicate::str::contains("completed"));

    // The record generated by 'done' should also be merged
    tascli()
        .env("TASCLI_TEST_DB", dest_path)
        .args(&["list", "record", "-d", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("finish report"));
}
