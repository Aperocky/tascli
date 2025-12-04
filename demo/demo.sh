#doitlive speed: 2
#doitlive prompt: {user.bold.cyan} $

# task operations
tascli task "create a simple demo task"
tascli task --category demo "create a recurring demo task" Daily
tascli task -c demo "demo task due end of month" eom
tascli list task
tascli list task today
tascli list task -c demo
tascli list task --search "demo task"
tascli done 1
tascli done 2
tascli update 3 -a "actually make it due Friday" -t friday
tascli list task -s all --search "demo task"

# record operations
echo "Now demo-ing inserting and listing records"
tascli record -c demo "Insert this record"
tascli record -c demo "if the content line of tasks and records gets long, it wraps around. 并且支持unicode字符"
tascli list record --search "demo task"
tascli list record -c demo
echo "Note how completed tasks automatically creates records"
tascli help

# cleanup.
#doitlive speed: 3
tascli list task -c demo -s all
yes | tascli delete 1
yes | tascli delete 2
tascli list task -s done --search "demo task"
yes | tascli delete 1
tascli list record --search "demo task"
yes | tascli delete 1
tascli list record -c demo
yes | tascli delete 1
yes | tascli delete 2
yes | tascli delete 3
