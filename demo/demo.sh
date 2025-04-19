#doitlive speed: 2
#doitlive prompt: {user.bold.cyan} $

# basic demo
tascli task "create simple task"
tascli list task
tascli list task today
tascli done 1

# task operations
tascli task -c demo "demo task due tomorrow 4:00PM" "tomorrow 4PM"
tascli task -c demo "demo task due end of month" eom
tascli list task -c demo
tascli update 1 -a "actually make it due Friday" -t friday
tascli done 2
tascli list task -c demo -s all

# record operations
tascli record "Insert this record" -c demo
tascli record "if the content line of tasks and records gets long, it wraps around. 并且支持unicode字符" -c demo
tascli list record -c demo
tascli list record -c FTP
tascli list record -s 3/22 -e 3/24

tascli help

# cleanup.
tascli list task -c demo -s all
tascli delete 1
tascli delete 2
tascli list task -c default
tascli delete 1
tascli list record -c demo
tascli delete 1
tascli delete 2
