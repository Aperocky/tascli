# tascli

A *simple* CLI tool for tracking tasks and records from terminal.

Installation:

```bash
# force for upgrade
cargo install tascli [--force]
```

## Basic Usage

Tasks and records are stored in `~/.local/share/tascli/tascli.db` with `rusqlite`.

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
------------------------------------------------------------------------------------------------------------------
| Index  | Category            | Content                                                   | Deadline            |
------------------------------------------------------------------------------------------------------------------
| 1      | default             | Create readme for tascli                                  | Today               |
------------------------------------------------------------------------------------------------------------------
| 2      | default             | Put tascli on crate.io                                    | Tomorrow            |
------------------------------------------------------------------------------------------------------------------
| 3      | default             | Do taxes                                                  | 4/15                |
------------------------------------------------------------------------------------------------------------------
| 4      | work                | Read work emails                                          | Tomorrow            |
------------------------------------------------------------------------------------------------------------------
```

Complete tasks:
```bash
# Mark index 1 as done
tascli done 1
```

List all tasks (including completed)
```bash
tascli list task --status all
```
output:
```
Task List:
------------------------------------------------------------------------------------------------------------------
| Index  | Category            | Content                                                   | Deadline            |
------------------------------------------------------------------------------------------------------------------
| 1      | default             | Create readme for tascli                                  | Today (completed)   |
------------------------------------------------------------------------------------------------------------------
| 2      | default             | Put tascli on crate.io                                    | Tomorrow            |
------------------------------------------------------------------------------------------------------------------
| 3      | default             | Do taxes                                                  | 4/15                |
------------------------------------------------------------------------------------------------------------------
| 4      | work                | Read work emails                                          | Tomorrow            |
------------------------------------------------------------------------------------------------------------------
```

### Records

Create records (for tracking events):
```bash
# With current time
tascli record -c feeding "100ML"

# With specific time
tascli record -c feeding -t 6:10PM "100ML"
```

List records:
```bash
tascli list record
```

output:
```
Records List:
------------------------------------------------------------------------------------------------------------------
| Index  | Category            | Content                                                   | Created At          |
------------------------------------------------------------------------------------------------------------------
| 1      | feeding             | 100ML                                                     | Today 6:10PM        |
------------------------------------------------------------------------------------------------------------------
| 2      | feeding             | 90ML                                                      | Today 9:30PM        |
------------------------------------------------------------------------------------------------------------------
```

### Help

`tascli` uses `clap` for argument parsing, use `--help` to get help on all levels of this cli:

```
aperocky@~$ tascli -h
Usage: tascli <COMMAND>

Commands:
  task    add task with end time
  record  add record
  done    Finish task or remove records
  update  Update tasks or records wording/deadlines
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
