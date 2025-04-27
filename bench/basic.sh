#!/bin/sh
# Task insertion benchmark
hyperfine -r 50 'tascli task -c benchmark "task performance benchmark"' -n "Task Insertion"
hyperfine -r 50 'tascli record -c benchmark "record performance benchmark"' -n "Record Insertion"
# List benchmark with 50 objects
hyperfine -r 50 'tascli list task -c benchmark' -n "List Tasks"
# Cleanup and deletion benchmark
hyperfine -r 50 'i=$((HYPERFINE_ITERATION % 50 + 1)); yes | tascli delete $i' -n "Task Deletion"
tascli list record -c benchmark >/dev/null
hyperfine -r 50 'i=$((HYPERFINE_ITERATION % 50 + 1)); yes | tascli delete $i' -n "Record Deletion"
