# Changelog

### v0.13.0
- ✨ Add `AGENTS.md` for agentic usage
- ⚙️  Add `time_format` for multiple display time formats
- ⚙️  Standard configuration file to be created on tool run.

### v0.12.0
- ✨ `tascli done today` to interactively complete all tasks due today
- ✨ `tascli done overdue` to interactively complete all tasks due today and prior
- ✨ `--comment` on `tascli update --add-content` now includes a timestamp moniker
- ✨ Interactive batch completions to support comments
- 🧪 Add integration tests for core workflows
- 📦 Bump rusqlite to 0.38

### v0.11.2
- 🐛 Fix list pagination bug where items with exact same timestamp become potentially omitted at the point of next token

### v0.11.1
- ✨ Added `--status` filter to batch command
- 🐛 Fix `--status-to` to only allow concrete (non-aggregate) statuses
- 📝 Update readme to include a section for ops

### v0.11.0
- ✨ Added `ops` commands for operations and visibility
- ✨ `ops stat` to have a holistic view of all of your tasks, records and recurring ones by category and time
- ✨ `ops batch` to batch update categories, status and due dates
- ✨ `ops backup` to backup your table (convenience function), also backups automatically for batch commands

### v0.10.2
- 🐛 Fix bug where pagination do not continue correctly from recurring after filtering and from recurring to regular tasks
- 📝 Minor readme and help string update

### v0.10.1
- ✨ Added `every` keyword to recurring tasks/schedule
- ⚡ Added performance benchmark for recurring tasks/records
- 📝 Update documentation related to recurring tasks/records

### v0.10.0
- ✨ Added recurring tasks and records for recurring tasks, use the same syntax but with a schedule, e.g. "Daily": `tascli task "write diary" daily`
- ✨ Completing recurring task will create a recurring task record. The recurring task would show as completed if queried until the next interval
- 📝 Update demo gif to include recurring tasks and records

### v0.9.0

- ✨ Added `tascli list show $index` command to simplify copy paste operations

### v0.8.0

- ✨ Completed task with `tascli done` will automatically populate a record of completion
- ✨ Add `--comment` flag to also add a comment on the task completion
- 📦 Update dependency versions for sqlite

### v0.7.0

- ✨ Add `--search` flag to list action to search content by text

### v0.6.1

- ⚡ Batch cache index transaction to further speed up list operations
- ⚡ Add performance benchmark to github workflows

### v0.6.0

- ⚙️  Add optional configuration file, allow db file to be placed in custom location
- ⚡ Add performance benchmark
- ⚡ Performance optimization on db connection
- 📝 Documentation updates including a README for benchmark
- 📦 Add `brew tap Aperocky/tascli` for `brew install tascli`

### v0.5.4

- 🎨 error output to be bright red.

### v0.5.3

- 📦 Remove regex dependency
- 📦 Reduce binary size with compilation flags; binary size now 1.5MB from 4.6MB

### v0.5.2

- 📝 Update command line help documentation
- 📝 Add demo script with doitlive
- 📦 Update rusqlite dependency

### v0.5.0

- ✨ Correctly space unicode characters in the table.
- 🏗️ Refactor display utility to a module, remove dependency on textwrap
- 📦 Bundle rusqlite (compiled size now 4.7MB)

### v0.4.0

- ✨ Add pagination for list task and list record with --next-page (-n)
- ⚡ Use sqlite order_by with index for list actions

### v0.3.0

- ✨ Add starting-time and ending-time optional argument for list records
- ✨ Add aggregate status 'open' and 'closed' for tasks
- ✨ Support query for aggregate status, use 'open' by default
- 📝 Add CHANGELOG.md

### v0.2.4

- 📝 Use more readme friendly width and update gif in documentation
- 🐛 Fix typo where creating record printed "inserted task" on output
- 🔄 update --add-content (-a) to add content to newline

### v0.2.3

- 🐛 Fix bug where done command output printed task as record

### v0.2.2

- 🔄 Sort list output by creation time for records and target time for tasks
- 📝 Add gif demo to documentation

### v0.2.1

- ✨ Use terminal_size to dynamically adjust to terminal width

### v0.2.0

- 🔒 Prevent common syntax mistakes by throwing errors
- ✨ New delete command to fully delete an item from db
- 🐛 Remove index from table output of insertion and update commands

### v0.1.0

- 🚀 Initial release of `tascli`
- ✨ Initial commands of task, record adding, listing & update & done
- ✨ Sqlite db module powered by `rusqlite`
- ✨ Dynamic timestr support for common time formatting
- ✨ Pretty table formatting
