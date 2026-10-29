#!/usr/bin/env bash
#
# Diagnostic stress-runner for the intermittent bdwgc
# "Duplicate large block deallocation" abort seen on macOS CI
# (examples/lifegame and examples/ray crash non-deterministically at GC time).
#
# Two phases per example:
#   1. plain loop  -> measures repro frequency and captures the GC abort message
#                     (which prints the faulting block address).
#   2. lldb loop   -> stops at the first crash and dumps a full backtrace of all
#                     threads so we can see which GC phase aborts.
#
# Iteration counts are env-tunable so we can adjust after seeing CI frequency.
# All phases always run so the logs are complete; the script then exits 1 if
# any run crashed, so the CI step fails.
#
# Usage: tools/gc_stress.sh <shiika-bin>

set -uo pipefail

BIN="${1:?usage: gc_stress.sh <shiika-bin>}"
PLAIN_ITERS="${PLAIN_ITERS:-500}"
LLDB_ITERS="${LLDB_ITERS:-60}"
RAY_PLAIN_ITERS="${RAY_PLAIN_ITERS:-3}"
RAY_LLDB_ITERS="${RAY_LLDB_ITERS:-3}"
TOTAL_CRASHES=0

compile() {
  echo "=== compiling examples/$1.sk ==="
  "$BIN" compile "examples/$1.sk"
}

# Plain loop: measure repro frequency and capture the GC abort message.
plain_stress() {
  local ex="$1" iters="$2" fails=0 i
  [ "$iters" -lt 1 ] && { echo "=== plain stress: $ex skipped (iters=$iters) ==="; return; }
  echo "=== plain stress: $ex x$iters ==="
  for i in $(seq 1 "$iters"); do
    ./examples/"$ex".out >/dev/null 2>/tmp/gc_err.txt
    if [ $? -ne 0 ]; then
      fails=$((fails + 1))
      echo "  [$ex] crash on iter $i: $(tail -2 /tmp/gc_err.txt | tr '\n' ' ')"
    fi
  done
  echo "=== plain stress: $ex -> $fails/$iters crashed ==="
  TOTAL_CRASHES=$((TOTAL_CRASHES + fails))
}

# lldb loop: stop at the first crash and dump a full backtrace.
lldb_stress() {
  local ex="$1" iters="$2" i
  [ "$iters" -lt 1 ] && { echo "=== lldb stress: $ex skipped (iters=$iters) ==="; return; }
  echo "=== lldb stress: $ex x$iters (stop on first crash) ==="
  for i in $(seq 1 "$iters"); do
    lldb --batch \
      -o 'run' \
      -k 'bt all' \
      -k 'register read' \
      -k 'quit 134' \
      -- ./examples/"$ex".out >/tmp/gc_lldb_out.txt 2>/tmp/gc_lldb_err.txt
    if [ $? -ne 0 ]; then
      echo "!!! $ex crashed under lldb on iter $i"
      TOTAL_CRASHES=$((TOTAL_CRASHES + 1))
      echo "----- lldb stdout -----"
      cat /tmp/gc_lldb_out.txt
      echo "----- lldb stderr -----"
      cat /tmp/gc_lldb_err.txt
      return 0
    fi
  done
  echo "=== lldb stress: $ex -> no crash in $iters runs ==="
}

compile lifegame
compile ray

plain_stress lifegame "$PLAIN_ITERS"
lldb_stress  lifegame "$LLDB_ITERS"

plain_stress ray "$RAY_PLAIN_ITERS"
lldb_stress  ray "$RAY_LLDB_ITERS"

echo "=== gc_stress done: $TOTAL_CRASHES crash(es) ==="
[ "$TOTAL_CRASHES" -eq 0 ]
