#!/bin/sh
# Composite benchmark: full workflow including task creation, completion, listing, and deletion

# Constants
RECURRING_TASKS=20
REGULAR_TASKS=50
TOTAL_TASKS=$((RECURRING_TASKS + REGULAR_TASKS))

hyperfine -r $REGULAR_TASKS 'tascli task -c composite_benchmark "composite_benchmark benchmark task"' -n "Regular Task Creation"

hyperfine -r $RECURRING_TASKS 'tascli task -c composite_benchmark "daily standup" "Daily 9AM"' -n "Recurring Task Creation"

hyperfine -r $REGULAR_TASKS 'tascli list task -c composite_benchmark --status all' -n "List Tasks"

tascli list task -c composite_benchmark --limit $RECURRING_TASKS >/dev/null
hyperfine -r $RECURRING_TASKS "i=\$((HYPERFINE_ITERATION % $RECURRING_TASKS + 1)); tascli done \$i" -n "Recurring Task Completion"

tascli list task -c composite_benchmark >/dev/null
hyperfine -r $REGULAR_TASKS "i=\$((HYPERFINE_ITERATION % $REGULAR_TASKS + 1)); tascli done \$i" -n "Regular Task Completion"

hyperfine -r $REGULAR_TASKS 'tascli list task -c composite_benchmark --status all' -n "List Completed Tasks"

hyperfine -r $TOTAL_TASKS "i=\$((HYPERFINE_ITERATION % $TOTAL_TASKS + 1)); yes | tascli delete \$i" -n "Delete Task"

hyperfine -r $REGULAR_TASKS 'tascli list record -c composite_benchmark' -n "List Records"

tascli list record -c composite_benchmark >/dev/null
hyperfine -r $TOTAL_TASKS "i=\$((HYPERFINE_ITERATION % $TOTAL_TASKS + 1)); yes | tascli delete \$i" -n "Delete Record"
