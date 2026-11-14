"""The README's animated hero: one design test recorded on a Googlebook-sized screen, a tablet and a
phone, played side by side at the same speed, so each finishes when it really did.

    python3 scripts/android-hero-race.py OUT_BASENAME \
        LAPTOP.mp4 START_S TOTAL_S "label" "note" \
        TABLET.mp4 START_S TOTAL_S "label" "note" \
        PHONE.mp4  START_S TOTAL_S "label" "note"

START_S is where the run starts in each screen recording and TOTAL_S how long it took (the test's
TOTAL line). Writes OUT_BASENAME.mp4 and OUT_BASENAME.gif. Needs Pillow and an ffmpeg (FFMPEG, or
the one on PATH)."""
import os, subprocess, sys, tempfile, glob
from PIL import Image, ImageDraw, ImageFont

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FFMPEG = os.environ.get("FFMPEG", "ffmpeg")
PLEX = f"{REPO}/crates/look/fonts/ibm-plex-sans/IBMPlexSans-Variable.ttf"
W, H, FPS, SPEED, HOLD_S = 1600, 900, 10, 3.0, 3.0
BG, FG, DIM, BEZEL, EDGE = (17, 19, 25), (236, 238, 242), (150, 156, 168), (34, 37, 45), (70, 75, 88)
GREEN, BLUE = (74, 222, 128), (88, 166, 255)


def font(size, weight="Regular"):
    f = ImageFont.truetype(PLEX, size)
    try:
        f.set_variation_by_name(weight)
    except Exception:
        pass
    return f


OUT = sys.argv[1]
args = sys.argv[2:]
devices = [dict(video=args[i], start=float(args[i + 1]), total=float(args[i + 2]), label=args[i + 3], note=args[i + 4]) for i in range(0, 15, 5)]
# Screen rectangles: a laptop, a tablet (both 16:10) and an upright phone.
devices[0]["screen"] = (60, 220, 640, 400)
devices[1]["screen"] = (770, 270, 480, 300)
devices[2]["screen"] = (1340, 195, 178, 400)

tmp = tempfile.mkdtemp()
for n, dev in enumerate(devices):
    x, y, w, h = dev["screen"]
    out = f"{tmp}/d{n}"
    os.makedirs(out)
    subprocess.run([FFMPEG, "-v", "error", "-ss", str(dev["start"]), "-t", str(dev["total"]), "-i", dev["video"],
                    "-vf", f"setpts=PTS/{SPEED},fps={FPS},scale={w}:{h}:flags=lanczos", f"{out}/%05d.png"], check=True)
    dev["frames"] = sorted(glob.glob(f"{out}/*.png"))

end_s = max(d["total"] for d in devices) / SPEED + HOLD_S
title, sub, small, timer = font(46, "Bold"), font(22), font(17), font(30, "SemiBold")


def frame_device(d, dev):
    x, y, w, h = dev["screen"]
    if dev is devices[0]:
        d.rounded_rectangle((x - 16, y - 16, x + w + 16, y + h + 16), 16, fill=BEZEL, outline=EDGE, width=2)
        d.polygon([(x - 56, y + h + 22), (x + w + 56, y + h + 22), (x + w + 30, y + h + 44), (x - 30, y + h + 44)], fill=(48, 52, 62))
        d.rounded_rectangle((x + w // 2 - 60, y + h + 22, x + w // 2 + 60, y + h + 28), 3, fill=(30, 33, 40))
    elif dev is devices[1]:
        d.rounded_rectangle((x - 18, y - 18, x + w + 18, y + h + 18), 22, fill=BEZEL, outline=EDGE, width=2)
    else:
        d.rounded_rectangle((x - 9, y - 22, x + w + 9, y + h + 22), 24, fill=BEZEL, outline=EDGE, width=2)
        d.ellipse((x + w // 2 - 4, y - 15, x + w // 2 + 4, y - 7), fill=(10, 10, 12))


tdir = f"{tmp}/out"
os.makedirs(tdir)
for k in range(int(end_s * FPS)):
    t_real = k / FPS * SPEED
    im = Image.new("RGB", (W, H), BG)
    d = ImageDraw.Draw(im)
    d.text((60, 40), "SolidCraft for Android and Googlebook OS", font=title, fill=FG)
    d.text((62, 104), "One cassette housing, 121 MCP calls, the same app on three screens. Played 3× faster; each finishes when it really did.",
           font=sub, fill=DIM)
    for dev in devices:
        frame_device(d, dev)
        x, y, w, h = dev["screen"]
        i = min(k, len(dev["frames"]) - 1)
        im.paste(Image.open(dev["frames"][i]).convert("RGB"), (x, y))
        done = t_real >= dev["total"]
        lx = x + w // 2
        ly = 690
        d.text((lx, ly), dev["label"], font=sub, fill=FG, anchor="mt")
        d.text((lx, ly + 30), dev["note"], font=small, fill=DIM, anchor="mt")
        shown = dev["total"] if done else t_real
        d.text((lx, ly + 60), ("done in " if done else "") + f"{shown:.1f} s", font=timer, fill=GREEN if done else BLUE, anchor="mt")
    d.text((60, H - 40), "Real timings from screen recordings of SolidCraft 0.41.0a. The laptop is the Android emulator standing in for a Googlebook (Googlebook OS is Android).",
           font=small, fill=DIM)
    im.save(f"{tdir}/{k:05d}.png")

subprocess.run([FFMPEG, "-v", "error", "-y", "-framerate", str(FPS), "-i", f"{tdir}/%05d.png", "-c:v", "libx264", "-crf", "24",
                "-preset", "slow", "-pix_fmt", "yuv420p", "-movflags", "+faststart", f"{OUT}.mp4"], check=True)
subprocess.run([FFMPEG, "-v", "error", "-y", "-framerate", str(FPS), "-i", f"{tdir}/%05d.png", "-vf",
                "scale=1200:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle",
                f"{OUT}.gif"], check=True)
print(f"{OUT}.mp4, {OUT}.gif: {end_s:.1f} s")
