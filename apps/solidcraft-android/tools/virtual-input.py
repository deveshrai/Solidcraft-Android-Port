#!/usr/bin/env python3
"""Drive a device or emulator with a virtual USB mouse, keyboard and multi-touch screen.

The emulator's own window only sends touches, and `adb shell input` has no right or middle
button, wheel or second finger. Android's `uinput` shell tool creates real input devices from
JSON commands read on stdin; this writes those commands to a file that is streamed to it:

    : > /tmp/ui.jsonl
    tail -f /tmp/ui.jsonl | adb shell uinput - &          # keep running while testing
    tools/virtual-input.py /tmp/ui.jsonl register         # mouse (id 1), keyboard (2), touch (3)

    tools/virtual-input.py /tmp/ui.jsonl move -1000 -1000  # relative mouse move (pointer
                                                           # acceleration applies, ~2x)
    tools/virtual-input.py /tmp/ui.jsonl btn right down    # left | right | middle, down | up
    tools/virtual-input.py /tmp/ui.jsonl wheel 1           # wheel clicks (+ away from you)
    tools/virtual-input.py /tmp/ui.jsonl key leftctrl down # hold a key (Linux KEY_ names)
    tools/virtual-input.py /tmp/ui.jsonl tap e             # press and release a key
    tools/virtual-input.py /tmp/ui.jsonl touch 900 800 1300 800   # fingers at screen pixels
    tools/virtual-input.py /tmp/ui.jsonl lift 2            # lift that many fingers
    tools/virtual-input.py /tmp/ui.jsonl delay 100         # wait (ms) before the next command

Screen pixels for `touch` are the display's: 2560x1600 (the Pixel Tablet AVD) unless
VIRTUAL_INPUT_SCREEN says otherwise (`VIRTUAL_INPUT_SCREEN=2448x1848`, from `adb shell wm size`),
set the same at `register` and after. Development tool only; nothing here ships in the app.
"""

import json
import os
import sys

SCREEN = tuple(int(n) for n in os.environ.get("VIRTUAL_INPUT_SCREEN", "2560x1600").lower().split("x", 1))
MOUSE, KEYBOARD, TOUCH = 1, 2, 3
SYN = ["EV_SYN", "SYN_REPORT", 0]
KEYS = [f"KEY_{c}" for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"] + [
    "KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_LEFTALT", "KEY_LEFTMETA", "KEY_ESC", "KEY_ENTER",
    "KEY_SPACE", "KEY_DELETE", "KEY_BACKSPACE", "KEY_TAB", "KEY_DOT", "KEY_MINUS",
    "KEY_UP", "KEY_DOWN", "KEY_LEFT", "KEY_RIGHT",
]


def axis(code, maximum):
    return {"code": code, "info": {"value": 0, "minimum": 0, "maximum": maximum, "fuzz": 0, "flat": 0, "resolution": 0}}


def main(argv):
    if len(argv) < 3:
        print(__doc__)
        return 2
    out = open(argv[1], "a")
    cmd, args = argv[2], argv[3:]

    def send(o):
        out.write(json.dumps(o) + "\n")

    def inject(dev, events):
        send({"id": dev, "command": "inject", "events": events + SYN})

    if cmd == "register":
        send({"id": MOUSE, "command": "register", "name": "Virtual Mouse", "vid": 6353, "pid": 1, "bus": "usb",
              "configuration": [{"type": "UI_SET_EVBIT", "data": ["EV_KEY", "EV_REL"]},
                                {"type": "UI_SET_KEYBIT", "data": ["BTN_LEFT", "BTN_RIGHT", "BTN_MIDDLE"]},
                                {"type": "UI_SET_RELBIT", "data": ["REL_X", "REL_Y", "REL_WHEEL", "REL_HWHEEL"]}]})
        send({"id": KEYBOARD, "command": "register", "name": "Virtual Keyboard", "vid": 6353, "pid": 2, "bus": "usb",
              "configuration": [{"type": "UI_SET_EVBIT", "data": ["EV_KEY"]}, {"type": "UI_SET_KEYBIT", "data": KEYS}]})
        send({"id": TOUCH, "command": "register", "name": "Virtual Touchscreen", "vid": 6353, "pid": 3, "bus": "usb",
              "configuration": [{"type": "UI_SET_EVBIT", "data": ["EV_KEY", "EV_ABS"]},
                                {"type": "UI_SET_KEYBIT", "data": ["BTN_TOUCH"]},
                                {"type": "UI_SET_ABSBIT", "data": ["ABS_MT_SLOT", "ABS_MT_TRACKING_ID",
                                                                   "ABS_MT_POSITION_X", "ABS_MT_POSITION_Y"]},
                                {"type": "UI_SET_PROPBIT", "data": ["INPUT_PROP_DIRECT"]}],
              "abs_info": [axis("ABS_MT_SLOT", 9), axis("ABS_MT_TRACKING_ID", 65535),
                           axis("ABS_MT_POSITION_X", SCREEN[0] - 1), axis("ABS_MT_POSITION_Y", SCREEN[1] - 1)]})
        send({"id": MOUSE, "command": "delay", "duration": 1500})
    elif cmd == "move":
        inject(MOUSE, ["EV_REL", "REL_X", int(args[0]), "EV_REL", "REL_Y", int(args[1])])
    elif cmd == "btn":
        inject(MOUSE, ["EV_KEY", "BTN_" + args[0].upper(), 1 if args[1] == "down" else 0])
    elif cmd == "wheel":
        inject(MOUSE, ["EV_REL", "REL_WHEEL", int(args[0])])
    elif cmd == "key":
        inject(KEYBOARD, ["EV_KEY", "KEY_" + args[0].upper(), 1 if args[1] == "down" else 0])
    elif cmd == "tap":
        for pressed in (1, 0):
            inject(KEYBOARD, ["EV_KEY", "KEY_" + args[0].upper(), pressed])
    elif cmd == "touch":
        points = [(int(args[i]), int(args[i + 1])) for i in range(0, len(args) - 1, 2)]
        events = []
        for slot, (x, y) in enumerate(points):
            events += ["EV_ABS", "ABS_MT_SLOT", slot, "EV_ABS", "ABS_MT_TRACKING_ID", 100 + slot,
                       "EV_ABS", "ABS_MT_POSITION_X", x, "EV_ABS", "ABS_MT_POSITION_Y", y]
        inject(TOUCH, events + ["EV_KEY", "BTN_TOUCH", 1])
    elif cmd == "lift":
        events = []
        for slot in range(int(args[0])):
            events += ["EV_ABS", "ABS_MT_SLOT", slot, "EV_ABS", "ABS_MT_TRACKING_ID", -1]
        inject(TOUCH, events + ["EV_KEY", "BTN_TOUCH", 0])
    elif cmd == "delay":
        send({"id": MOUSE, "command": "delay", "duration": int(args[0])})
    else:
        print(f"unknown command: {cmd}\n\n{__doc__}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
