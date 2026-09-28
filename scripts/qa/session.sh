#!/bin/bash
# Scripted interaction with an example on the Xvfb display (X11 backend):
# start it, perform actions, screenshot, kill it. Run one at a time.
#
#   scripts/qa/session.sh EXAMPLE OUT_PREFIX "action;action;..."
#
# actions:  wait:SECONDS   shot:NAME   click:X,Y   drag:X1,Y1,X2,Y2
# e.g.      scripts/qa/session.sh example_1 /tmp/e1 "shot:0;click:30,335;wait:1;shot:checked"
# writes    /tmp/e1.0.png, /tmp/e1.checked.png and the app's output to /tmp/e1.log
# Python is unbuffered so the log shows what the callbacks print (handy for
# counting callback invocations).
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
cd "$ROOT" && . ./env.sh >/dev/null
"$HERE/xvfb_start"
EX=$1; OUT=$2; ACTS=$3
env -u WAYLAND_DISPLAY DISPLAY=:99 python -u "examples/$EX.py" > "$OUT.log" 2>&1 &
PID=$!
sleep 6
IFS=';' read -ra A <<< "$ACTS"
for a in "${A[@]}"; do
  k=${a%%:*}; v=${a#*:}
  case $k in
    wait) sleep "$v";;
    shot) python "$HERE/shot.py" "$OUT.$v.png";;
    click) IFS=, read -r x y <<< "$v"; python "$HERE/input.py" click "$x" "$y";;
    drag) IFS=, read -r x1 y1 x2 y2 <<< "$v"; python "$HERE/input.py" drag "$x1" "$y1" "$x2" "$y2";;
  esac
done
kill $PID 2>/dev/null; wait $PID 2>/dev/null
exit 0
