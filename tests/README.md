# Integration Tests

This directory contains integration tests for tascli that test end-to-end behavior by running the actual binary on a test db.

The integration tests are mostly implemented for command interactions that is difficult to reproduce in unit tests. The unit test already include actual DB tests using temp files.

## Overview

**Current tests:**
- `basic_operations.rs` - Tests basic CRUD operations (create, list, update, delete, done)
- `interactive_done.rs` - Tests interactive batch completion flows (`done today`, `done overdue`)
- `interactive_batch.rs` - Tests interactive batch operations (`ops batch --interactive`)

## Running Tests

### Run all integration tests
```bash
cargo test --test '*'
```

### Run specific test file
```bash
cargo test --test interactive_done
```

### Run a specific test
```bash
cargo test --test interactive_done test_done_today_complete_all_tasks
```

### Run only unit tests (not tests here)
```bash
cargo test --bin tascli
```
