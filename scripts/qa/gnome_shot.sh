#!/bin/bash
# Run an app inside a private headless GNOME Shell (real Wayland compositor,
# real GNOME window decorations) and screenshot it via the shell's own API.
# usage: gnome_shot.sh PYTHON EXAMPLE.py OUT.png [SECONDS=7]
# Starts the shell on first use (bus address in /tmp/gs/bus.txt).
PY=$1; EX=$2; OUT=$3; SECS=${4:-7}
mkdir -p /tmp/gs
if ! pgrep -f "gnome-shell --headless.*qa-0" >/dev/null; then
  dbus-daemon --session --fork --print-address > /tmp/gs/bus.txt 2>/dev/null
  DBUS_SESSION_BUS_ADDRESS=$(head -1 /tmp/gs/bus.txt) WAYLAND_DISPLAY= nohup gnome-shell --headless --wayland --no-x11 \
    --virtual-monitor 1700x1100 --wayland-display=qa-0 --unsafe-mode > /tmp/gs/shell.log 2>&1 &
  sleep 6
fi
export DBUS_SESSION_BUS_ADDRESS=$(head -1 /tmp/gs/bus.txt)
env WAYLAND_DISPLAY=qa-0 DISPLAY= "$PY" "$EX" > "${OUT%.png}.log" 2>&1 &
PID=$!
sleep "$SECS"
gdbus call --session --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot \
  --method org.gnome.Shell.Screenshot.Screenshot false false "$OUT" >/dev/null
kill $PID 2>/dev/null; wait $PID 2>/dev/null
