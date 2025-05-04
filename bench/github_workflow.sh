#!/bin/bash

cd "$(dirname "$0")"
./basic.sh >output.txt 2>/dev/null

declare -A standards
standards["Task Insertion"]=10
standards["Record Insertion"]=10
standards["List Tasks"]=10
standards["Task Deletion"]=10
standards["Record Deletion"]=10
failed=0

while IFS='|' read -r name time; do
    standard=${standards["$name"]}
    if (($(echo "$time > $standard" | bc -l))); then
        echo "❌ FAIL: $name - $time ms (exceeds limit of $standard ms)"
        failed=1
    else
        echo "✅ PASS: $name - $time ms (within limit of $standard ms)"
    fi
done < <(awk '/Benchmark 1:/{name=$0; sub(/.*Benchmark 1: /,"",name)} /Time \(mean/{print name "|" $5}' output.txt)

exit $failed
