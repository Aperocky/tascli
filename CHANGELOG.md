# Changelog

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
