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
# The script exits 1 as soon as any run crashes, so the CI step fails.
#
# Usage: tools/gc_stress.sh <shiika-bin>

set -uo pipefail

BIN="${1:?usage: gc_stress.sh <shiika-bin>}"
PLAIN_ITERS="${PLAIN_ITERS:-500}"
LLDB_ITERS="${LLDB_ITERS:-60}"
RAY_PLAIN_ITERS="${RAY_PLAIN_ITERS:-3}"
RAY_LLDB_ITERS="${RAY_LLDB_ITERS:-3}"

compile() {
  echo "=== compiling examples/$1.sk ==="
  "$BIN" compile "examples/$1.sk"
}

# Plain loop: measure repro frequency and capture the GC abort message.
plain_stress() {
  local ex="$1" iters="$2" i
  [ "$iters" -lt 1 ] && { echo "=== plain stress: $ex skipped (iters=$iters) ==="; return; }
  echo "=== plain stress: $ex x$iters ==="
  for i in $(seq 1 "$iters"); do
    ./examples/"$ex".out >/dev/null 2>/tmp/gc_err.txt
    if [ $? -ne 0 ]; then
      echo "!!! $ex crashed on iter $i: $(tail -2 /tmp/gc_err.txt | tr '\n' ' ')"
      exit 1
    fi
  done
  echo "=== plain stress: $ex -> no crash in $iters runs ==="
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
      echo "----- lldb stdout -----"
      cat /tmp/gc_lldb_out.txt
      echo "----- lldb stderr -----"
      cat /tmp/gc_lldb_err.txt
      exit 1
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

echo "=== gc_stress done ==="
exit 0
