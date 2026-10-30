"""The README's hero image: the port next to upstream, with a screenshot of the latest device test.

    python3 scripts/android-hero.py SCREENSHOT.png docs/screenshots/android/hero-port-vs-upstream.png 0.27.1 0.27.1a

Needs Pillow. Edit the call-out and caption text below for each release's test."""
import sys
from PIL import Image, ImageDraw, ImageFont

REPO = __import__("os").path.dirname(__import__("os").path.dirname(__import__("os").path.abspath(__file__)))
SHOT, OUT, UP, PORT = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
W, H = 1800, 960
BG, FG, DIM, LINE = (17, 19, 25), (236, 238, 242), (160, 166, 178), (52, 56, 66)
BLUE, GREEN, RED, YELLOW = (88, 166, 255), (74, 222, 128), (248, 81, 73), (250, 204, 21)
PLEX = f"{REPO}/assets/fonts/ibm-plex-sans/IBMPlexSans-Variable.ttf"


def font(size, weight="Regular"):
    f = ImageFont.truetype(PLEX, size)
    try:
        f.set_variation_by_name(weight)
    except Exception:
        pass
    return f


im = Image.new("RGB", (W, H), BG)
d = ImageDraw.Draw(im)
d.text((60, 42), "SolidCraft for Android", font=font(60, "Bold"), fill=FG)
d.text((62, 116), "Upstream SolidCraft, on phones, tablets and foldables, plus what upstream doesn't have yet.", font=font(22), fill=DIM)

# The device screenshot in a dark rounded frame.
shot = Image.open(SHOT).convert("RGB")
sw = 900
shot = shot.resize((sw, round(shot.height * sw / shot.width)), Image.LANCZOS)
fx, fy, pad = 46, 150, 14
frame = (fx, fy, fx + sw + 2 * pad, fy + shot.height + 2 * pad)
d.rounded_rectangle(frame, radius=26, fill=(36, 39, 48))
mask = Image.new("L", shot.size, 0)
ImageDraw.Draw(mask).rounded_rectangle((0, 0, *shot.size), radius=12, fill=255)
im.paste(shot, (fx + pad, fy + pad), mask)

# Call-out over the screenshot: what the run did.
cx, cy = fx + pad + 170, fy + pad + 452
box = (cx, cy, cx + 410, cy + 66)
d.rounded_rectangle(box, radius=10, fill=(255, 236, 160), outline=YELLOW, width=3)
d.text((cx + 14, cy + 6), "220 mm M10 lead screw: 1.2 s", font=font(20, "SemiBold"), fill=(30, 30, 30))
d.text((cx + 14, cy + 34), "helical B-rep · whole stage over MCP in 68 s", font=font(16), fill=(60, 60, 60))

d.text((60, fy + shot.height + 2 * pad + 22),
       "A linear stage (guide rods, bearing blocks, lead screw, nut, coupling, motor), built by an agent",
       font=font(20), fill=DIM)
d.text((60, fy + shot.height + 2 * pad + 50), "over the MCP server on a Galaxy Tab: 52 operations and every check in 68 s.", font=font(20), fill=DIM)

# The comparison table.
tx, c1, c2 = 1060, 1530, 1670
d.text((c1, 170), "Upstream", font=font(22, "SemiBold"), fill=DIM, anchor="ma")
d.text((c1, 198), UP, font=font(20), fill=DIM, anchor="ma")
d.text((c2, 170), "This port", font=font(22, "SemiBold"), fill=BLUE, anchor="ma")
d.text((c2, 198), PORT, font=font(20), fill=BLUE, anchor="ma")


def tick(x, y):
    d.line([(x - 14, y), (x - 4, y + 11), (x + 15, y - 13)], fill=GREEN, width=5, joint="curve")


def cross(x, y):
    d.line([(x - 11, y - 11), (x + 11, y + 11)], fill=RED, width=4)
    d.line([(x - 11, y + 11), (x + 11, y - 11)], fill=RED, width=4)


rows = [
    ("Android app", "touch, mouse and keyboard, file picker", None, True),
    ("Helical B-rep threads", "exact, any pitch, long lead screws, phase", "faceted", True),
    ("Involute gears", "ten types, bore, keyway, hub", None, True),
    ("MCP server on the network", "Claude Code from Termux or Wi-Fi", "stdio", True),
    ("Every upstream feature", "same commands, kept in step", True, True),
]
y = 232
for name, sub, up, port in rows:
    d.line([(tx, y), (1750, y)], fill=LINE, width=2)
    d.text((tx, y + 16), name, font=font(26, "SemiBold"), fill=FG)
    d.text((tx, y + 54), sub, font=font(19), fill=DIM)
    mid = y + 46
    if up is True:
        tick(c1, mid)
    elif up is None:
        cross(c1, mid)
    else:
        d.text((c1, mid), up, font=font(20), fill=DIM, anchor="mm")
    tick(c2, mid)
    y += 112
d.line([(tx, y), (1750, y)], fill=LINE, width=2)
d.text((tx, y + 28), "github.com/deveshrai/Solidcraft-Android-Port", font=font(22, "SemiBold"), fill=BLUE)
d.text((tx, y + 60), "Open source · pure Rust · releases with APKs", font=font(19), fill=DIM)
im.save(OUT, optimize=True)
print(OUT, im.size)
