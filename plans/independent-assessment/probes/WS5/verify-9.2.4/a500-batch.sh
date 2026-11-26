#!/usr/bin/env bash
v=build/tmp/assessment/verify
bash $v/run-a500.sh c541 $v/a500-seq/fault $v/stage/v541c-13/prog $v/stage/v541c-13/walker2-13
CLICK=110 T=200 bash $v/run-a500.sh alert $v/a500-seq/fault $v/stage/alert13-13/prog $v/stage/alert13-13/walker2-13
CLICK=110 T=200 bash $v/run-a500.sh alertc $v/a500-seq/fault $v/stage/alert13c-13/prog $v/stage/alert13c-13/walker2-13
