"""A screen recording from a foldable's inner screen, shown in the frame of an open folding phone, with
a caption: for the README.

    python3 scripts/android-fold-frame.py RECORDING.mp4 START_S OUT_BASENAME "Caption" "Smaller line"

Writes OUT_BASENAME.mp4 and OUT_BASENAME.gif. Needs Pillow and an ffmpeg (FFMPEG, or the one on
PATH)."""
import glob, os, subprocess, sys, tempfile
from PIL import Image, ImageDraw, ImageFont

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
FFMPEG = os.environ.get("FFMPEG", "ffmpeg")
PLEX = f"{REPO}/crates/look/fonts/ibm-plex-sans/IBMPlexSans-Variable.ttf"
VIDEO, START, OUT, CAPTION, SMALL = sys.argv[1], float(sys.argv[2]), sys.argv[3], sys.argv[4], sys.argv[5]
SW, SH = 1060, 800          # the inner screen, as the recording's 2448 x 1848
BEZEL, FPS = 20, 8
W, H = SW + 2 * BEZEL + 120, SH + 2 * BEZEL + 170
BG, BODY, EDGE, CREASE, FG, DIM = (17, 19, 25), (30, 33, 40), (72, 77, 90), (44, 48, 58), (236, 238, 242), (150, 156, 168)


def font(size, weight="Regular"):
    f = ImageFont.truetype(PLEX, size)
    try:
        f.set_variation_by_name(weight)
    except Exception:
        pass
    return f


tmp = tempfile.mkdtemp()
subprocess.run([FFMPEG, "-v", "error", "-ss", str(START), "-i", VIDEO, "-vf", f"fps={FPS},scale={SW}:{SH}:flags=lanczos", f"{tmp}/%05d.png"], check=True)
frames = sorted(glob.glob(f"{tmp}/*.png"))
x0, y0 = (W - SW) // 2, 40
base = Image.new("RGB", (W, H), BG)
d = ImageDraw.Draw(base)
# The open phone: a body with rounded corners, the hinge's crease down the middle, the hinge's
# ends showing at the top and bottom (the inner screen's camera is under the display).
d.rounded_rectangle((x0 - BEZEL, y0 - BEZEL, x0 + SW + BEZEL, y0 + SH + BEZEL), 34, fill=BODY, outline=EDGE, width=3)
cx = x0 + SW // 2
for y in (y0 - BEZEL - 6, y0 + SH + BEZEL - 4):
    d.rounded_rectangle((cx - 14, y, cx + 14, y + 10), 4, fill=EDGE)
d.text((W // 2, y0 + SH + BEZEL + 34), CAPTION, font=font(28, "SemiBold"), fill=FG, anchor="mt")
d.text((W // 2, y0 + SH + BEZEL + 76), SMALL, font=font(20), fill=DIM, anchor="mt")
out = f"{tmp}/out"
os.makedirs(out)
for i, f in enumerate(frames + [frames[-1]] * (FPS * 3)):
    im = base.copy()
    im.paste(Image.open(f).convert("RGB"), (x0, y0))
    dd = ImageDraw.Draw(im)
    dd.line((cx, y0, cx, y0 + SH), fill=CREASE, width=2)
    im.save(f"{out}/{i:05d}.png")
subprocess.run([FFMPEG, "-v", "error", "-y", "-framerate", str(FPS), "-i", f"{out}/%05d.png", "-c:v", "libx264", "-crf", "24", "-preset", "slow",
                "-pix_fmt", "yuv420p", "-movflags", "+faststart", f"{OUT}.mp4"], check=True)
subprocess.run([FFMPEG, "-v", "error", "-y", "-framerate", str(FPS), "-i", f"{out}/%05d.png", "-vf",
                "scale=960:-1:flags=lanczos,split[a][b];[a]palettegen=max_colors=128:stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4:diff_mode=rectangle",
                f"{OUT}.gif"], check=True)
print(f"{OUT}.mp4, {OUT}.gif")
