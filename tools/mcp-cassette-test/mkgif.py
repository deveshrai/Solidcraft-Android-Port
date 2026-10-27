"""mkgif.py FRAMES_DIR TIMINGS.jsonl|- OUT.gif TITLE [width] [speed] [captions.json]
Frames are named <unix time>.png; each gets the operation running then as its caption."""
import sys, os, json, glob
from PIL import Image, ImageDraw, ImageFont

fd, tl, out, title = sys.argv[1:5]
W = int(sys.argv[5]) if len(sys.argv) > 5 else 720
SPEED = float(sys.argv[6]) if len(sys.argv) > 6 else 4.0
FONT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "assets", "fonts", "ibm-plex-sans", "IBMPlexSans-Variable.ttf")
manual = json.load(open(sys.argv[7])) if len(sys.argv) > 7 else None

frames = []
for p in glob.glob(os.path.join(fd, "*.png")):
    if os.path.getsize(p) < 10000:
        continue
    frames.append((float(os.path.basename(p)[:-4]), p))
frames.sort()
times = [t for t, _ in frames]
ops = [json.loads(l) for l in open(tl)] if tl != "-" else []


def caption(t):
    if manual:
        c = ""
        for off, text in manual:
            if t - times[0] >= off:
                c = text
        return c
    cur = None
    for o in ops:
        if "t0" in o and o["t0"] <= t <= o["t0"] + o["s"] + 0.4:
            cur = o
    if cur is None:
        done = [o for o in ops if "t0" in o and o["t0"] + o["s"] < t]
        if not done:
            return "starting…"
        cur = done[-1]
    n = ops.index(cur) + 1
    return f"{n}/{len(ops)}  {cur['label']}  ·  {cur['s']:.2f} s"


f_title = ImageFont.truetype(FONT, 22)
f_cap = ImageFont.truetype(FONT, 19)
try:
    f_title.set_variation_by_name("SemiBold")
except Exception:
    pass
imgs, durs = [], []
for i, (t, p) in enumerate(frames):
    try:
        im = Image.open(p).convert("RGB")
    except Exception:
        continue
    h = round(im.height * W / im.width)
    im = im.resize((W, h), Image.LANCZOS)
    bar = 34
    canvas = Image.new("RGB", (W, h + 2 * bar), (22, 24, 31))
    canvas.paste(im, (0, bar))
    d = ImageDraw.Draw(canvas)
    d.text((12, 5), title, font=f_title, fill=(235, 238, 245))
    d.text((W - 12, 7), f"{t - times[0]:5.1f} s", font=f_cap, fill=(140, 200, 255), anchor="ra")
    d.text((12, h + bar + 7), caption(t), font=f_cap, fill=(255, 214, 120))
    imgs.append(canvas.quantize(colors=128, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE))
    nxt = frames[i + 1][0] if i + 1 < len(frames) else t + 1.0
    durs.append(int(min(max((nxt - t) / SPEED, 0.08), 0.9) * 1000))
durs[-1] = 3000
imgs[0].save(out, save_all=True, append_images=imgs[1:], duration=durs, loop=0, optimize=True, disposal=1)
print(out, len(imgs), "frames", round(os.path.getsize(out) / 1e6, 2), "MB", round(sum(durs) / 1000, 1), "s")
