# tascli — Agent Usage Guide

`tascli` is a CLI tool for tracking tasks and records. Data is stored locally in SQLite at `~/.local/share/tascli/tascli.db`.

## Core Concepts

- **Task**: something to be done, has a deadline and a status. Completing a task (`done`) automatically creates a record.
- **Record**: a log entry of something that happened, has a timestamp. Records are append-only observations.
- **Category**: optional grouping label for both tasks and records (`-c`).

---

## Tasks

### Add a task
```sh
tascli task "description"                        # due end of today
tascli task "description" friday                 # due this Friday
tascli task "description" "2026/03/20 3PM"       # due specific datetime
tascli task "description" -c work                # with category
tascli task "description" "Daily 9AM" -c work    # recurring task
```

**Time strings**: `today`, `tomorrow`, `friday`, `eod`, `eom`, `eoy`, `2026/03/20`, `3PM`, `tomorrow 9AM`, `2026/03/20 14:00`

**Recurring schedules**: `Daily 9AM`, `Weekly Monday 9AM`, `Weekly Monday-Friday 3PM`, `Monthly 1st 9AM`, `Yearly 7/4 12PM`

### List tasks
```sh
tascli list task                   # open tasks (default)
tascli list task today             # due today
tascli list task -c work           # by category
tascli list task -s all            # all statuses
tascli list task -s done           # completed tasks
tascli list task --search "text"   # search content
tascli list task --overdue         # show overdue tasks
```

**Statuses**: `ongoing`, `done`, `cancelled`, `duplicate`, `suspended`, `pending`, `open`, `closed`, `all`

### Complete a task
```sh
tascli list task                        # get index first
tascli done 1                           # complete by index
tascli done 1 -c "optional comment"    # with comment
tascli done today                       # interactively complete all tasks due today
tascli done overdue                     # interactively complete all overdue tasks
```

### Update a task
```sh
tascli update 1 -t friday              # change deadline
tascli update 1 -c work                # change category
tascli update 1 -w "new description"   # replace content
tascli update 1 -a "appended note"     # append to content
tascli update 1 -s suspended           # change status
```

### Delete
```sh
tascli list task -s all --search "text"   # find it first
yes | tascli delete 1                     # delete by index (yes skips confirmation)
```

---

## Records

### Add a record
```sh
tascli record "what happened"              # timestamped now
tascli record "what happened" -c feeding   # with category
tascli record "what happened" -t "2PM"     # at a specific time
tascli record "what happened" -t "yesterday 3PM"
```

### List records
```sh
tascli list record                         # recent records
tascli list record -c feeding              # by category
tascli list record -d 7                    # last 7 days
tascli list record -s "yesterday" -e "today"   # time range
tascli list record --search "text"         # search content
```

---

## Workflow Pattern

The typical agent workflow for task tracking:

1. **Search first**: `tascli list task --search "keyword"` — check for duplicates before creating
2. **Create if new**: `tascli task "do something" friday -c work`
3. **Fresh list before mutating**: `tascli list task today` — indexes are ephemeral and reset on each list, always re-list before `done`, `update`, or `delete`
4. **Complete**: `tascli done <index>` — this also creates a record automatically
5. **Log directly**: `tascli record "observed something" -c work` — for things without a prior task; search with `tascli list record --search "keyword"` first to avoid duplicate records
