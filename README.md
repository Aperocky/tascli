# tascli

[![Crates.io](https://img.shields.io/crates/v/tascli.svg)](https://crates.io/crates/tascli)
[![tests](https://github.com/Aperocky/tascli/workflows/run%20tests/badge.svg)](https://github.com/Aperocky/tascli/actions?query=workflow%3Atests)
![Downloads](https://img.shields.io/crates/d/tascli.svg)

A *simple, fast, local* CLI tool for tracking tasks and records from unix terminal.

Installation:

```bash
cargo install tascli
# or use brew
brew tap Aperocky/tascli
brew install tascli
```

![tascli demo](demo/tascli.gif)

## Basic Usage

Tasks and records are stored in `~/.local/share/tascli/tascli.db` (configurable) with `rusqlite`.

### Tasks

Create tasks with deadlines:
```bash
# Basic tasks
tascli task "Create readme" today
tascli task "Publish package" tomorrow
tascli task "Do taxes" 4/15

# With category
tascli task -c work "Read emails" week
```

List tasks:
```bash
# List active tasks
$ tascli list task
```
output:
```
Task List:
----------------------------------------------------------------------------------------------
| Index  | Category            | Content                               | Deadline            |
----------------------------------------------------------------------------------------------
| 1      | life                | Unpack the crib                       | Today               |
----------------------------------------------------------------------------------------------
| 2      | tascli              | Add pagination capability for tascli  | Sunday              |
|        |                     | list actions                          |                     |
----------------------------------------------------------------------------------------------
| 3      | tascli              | Add readme section on timestring      | Sunday              |
|        |                     | format                                |                     |
----------------------------------------------------------------------------------------------
| 4      | life                | Do state taxes                        | Sunday              |
----------------------------------------------------------------------------------------------
| 5      | tascli              | Sort list output by time instead of   | Sunday              |
|        |                     | internal id                           |                     |
----------------------------------------------------------------------------------------------
| 6      | tascli              | Fix length issue for unicode chars    | Sunday              |
----------------------------------------------------------------------------------------------
| 7      | life                | Two month pictures - follow the lead  | 4/23                |
|        |                     | from the previous one month pictures  |                     |
----------------------------------------------------------------------------------------------
```

Complete tasks:
```bash
# Mark index 1 as done
tascli done 1
```

List all tasks in `tascli` category (including completed)
```bash
tascli list task -s all -c tascli
```
output:
```
Task List:
----------------------------------------------------------------------------------------------
| Index  | Category            | Content                               | Deadline            |
----------------------------------------------------------------------------------------------
| 1      | tascli              | Add a tascli command to delete a row  | Today (completed)   |
|        |                     | in the task or record table           |                     |
----------------------------------------------------------------------------------------------
| 2      | tascli              | Fix addition and modification commands| Today (completed)   |
|        |                     | output to have N/A for index          |                     |
----------------------------------------------------------------------------------------------
| 3      | tascli              | Insert guardrail against accidental   | Today (completed)   |
|        |                     | valid syntax like 'task list' that is |                     |
|        |                     | mistakenly made                       |                     |
----------------------------------------------------------------------------------------------
| 4      | tascli              | Create a gif for readme               | Today (completed)   |
----------------------------------------------------------------------------------------------
| 5      | tascli              | Add pagination capability for tascli  | Sunday              |
|        |                     | list actions                          |                     |
----------------------------------------------------------------------------------------------
| 6      | tascli              | Add readme section on timestring      | Sunday              |
|        |                     | format                                |                     |
----------------------------------------------------------------------------------------------
```

### Records

Create records (for tracking events):
```bash
# With current time
tascli record -c feeding "100ML"

# With specific time
tascli record -c feeding -t 11:20AM "100ML"
```

List records:
```bash
# -d 1 stand for only get last 1 day of record
tascli list record -d 1
```

output:
```
Records List:
----------------------------------------------------------------------------------------------
| Index  | Category            | Content                               | Created At          |
----------------------------------------------------------------------------------------------
| 1      | feeding             | 110ML                                 | Today 1:00AM        |
----------------------------------------------------------------------------------------------
| 2      | feeding             | breastfeeding                         | Today 4:10AM        |
----------------------------------------------------------------------------------------------
| 3      | feeding             | 100ML                                 | Today 7:30AM        |
----------------------------------------------------------------------------------------------
| 4      | feeding             | 110ML                                 | Today 11:20AM       |
----------------------------------------------------------------------------------------------
```

### Time Format

This application accepts flexible time strings in various formats:

- **Simple dates**: `today`, `tomorrow`, `yesterday`, `friday`, `eom` (end of month), `eoy` (end of year)
- **Date formats**: `YYYY-MM-DD`, `MM/DD/YYYY`, `MM/DD` (current year)
- **Time formats**: `HH:MM`, `3:00PM`, `3PM`
- **Combined**: `2025-03-24 15:30`, `tomorrow 3PM`

When only a date is provided, the time defaults to end of day (23:59:59). When only a time is provided, the date defaults to today.

### Configuration

If storing the db file in location other than `~/.local/share/tascli/tascli.db` is preferred, create a config file:

```
{
    "data_dir": "/where/you/want/it"
}
```

at `~/.config/tascli/config.json` to adjust the location of the stored file. Note, if you already have existing tasks, you may want to move/copy the db file there first.

### Help

`tascli` uses `clap` for argument parsing, use `--help` to get help on all levels of this cli:

```
aperocky@~$ tascli -h
Usage: tascli <COMMAND>

Commands:
  task    add task with end time
  record  add record
  done    Finish tasks
  update  Update tasks or records wording/deadlines
  delete  Delete Records or Tasks
  list    list tasks or records
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
aperocky@~$ tascli task -h
add task with end time

Usage: tascli task [OPTIONS] <CONTENT> [TIMESTR]

Arguments:
  <CONTENT>  Description of the task
  [TIMESTR]  Time the task is due, default to EOD

Options:
  -c, --category <CATEGORY>  Category of the task
  -h, --help                 Print help
```
