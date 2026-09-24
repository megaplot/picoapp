#!/bin/bash
# usage: run_example.sh PYTHON EXAMPLE.py OUT.png [SECONDS=6]
# Runs an example on the Xvfb display :99 (X11 backend), screenshots it, kills it.
# Start Xvfb first:  scripts/qa/xvfb_start
PY=$1; EX=$2; OUT=$3; SECS=${4:-6}
HERE=$(cd "$(dirname "$0")" && pwd)
env -u WAYLAND_DISPLAY DISPLAY=:99 "$PY" "$EX" > "${OUT%.png}.log" 2>&1 &
PID=$!
sleep "$SECS"
python "$HERE/shot.py" "$OUT" :99
kill $PID 2>/dev/null; wait $PID 2>/dev/null
