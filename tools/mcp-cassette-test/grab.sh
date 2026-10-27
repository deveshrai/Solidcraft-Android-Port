#!/bin/sh
# grab.sh SERIAL OUTDIR [DISPLAY_ID] : screenshots as fast as the device gives them, until OUTDIR/stop exists
mkdir -p "$2"; rm -f "$2/stop"
while [ ! -f "$2/stop" ]; do
  t=$(python3 -c 'import time;print(f"{time.time():.3f}")')
  if [ -n "$3" ]; then adb -s "$1" exec-out screencap -d "$3" -p > "$2/$t.png"; else adb -s "$1" exec-out screencap -p > "$2/$t.png"; fi
done
