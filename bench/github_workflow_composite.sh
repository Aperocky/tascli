#!/bin/bash

cd "$(dirname "$0")"
./composite.sh >output_composite.txt 2>/dev/null

declare -A standards
standards["Regular Task Creation"]=10
standards["Recurring Task Creation"]=10
standards["List Tasks"]=15
standards["Recurring Task Completion"]=10
standards["Regular Task Completion"]=10
standards["List Completed Tasks"]=15
standards["Delete Task"]=10
standards["List Records"]=15
standards["Delete Record"]=10
failed=0

while IFS='|' read -r name time; do
    standard=${standards["$name"]}
    if (($(echo "$time > $standard" | bc -l))); then
        echo "❌ FAIL: $name - $time ms (exceeds limit of $standard ms)"
        failed=1
    else
        echo "✅ PASS: $name - $time ms (within limit of $standard ms)"
    fi
done < <(awk '/Benchmark 1:/{name=$0; sub(/.*Benchmark 1: /,"",name)} /Time \(mean/{print name "|" $5}' output_composite.txt)

exit $failed
