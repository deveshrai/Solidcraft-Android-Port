# MCP cassette test

A live design test of the Android port's MCP server: it builds a compact-cassette housing with five
threaded M2 screws on a device, only through the server, times every call and checks that the screws
fit. Results and recordings: [docs/android-mcp-live-test.md](../../docs/android-mcp-live-test.md).

| File | What it does |
|---|---|
| `mcp.py` | A small MCP client for the server's HTTP transport; times every call into a JSON-lines log |
| `cassette.py` | The test: both halves, the tape-path block, holes, tapped threads, the screw, the fit, the checks, exports |
| `fit.py` | The thread fit from two exported STL meshes: the smallest radial gap at every turn of the screw |
| `grab.sh` | Screenshots from the device as fast as it gives them, while a run goes on |
| `mkgif.py` | A captioned GIF from those screenshots and the timings log |

## Run it

1. On the device: **Settings › MCP server**, tick **Turn on the MCP server**, choose **Another computer**,
   and note the address and the token (Advanced › Show).
2. On a computer on the same Wi-Fi, with adb connected to the device (the fit check copies the exported
   meshes from it) and numpy installed:

   ```sh
   cd tools/mcp-cassette-test
   MCP_URL=http://PHONE:27880/mcp MCP_TOKEN=... ADB_SERIAL=... MCP_LOG=mydevice.jsonl \
       python3 cassette.py mydevice
   ```

   It prints each call with its time, the fit of every screw and the checks, and leaves screenshots
   (`mydevice-*.png`) here and the design, STEP and 3MF in the agent's folder on the device.

To record it: start `./grab.sh SERIAL frames [DISPLAY_ID]` before the run, `touch frames/stop` after, then
`python3 mkgif.py frames mydevice.jsonl out.gif "Title"` (needs Pillow). A foldable's inner screen has its
own display id (`adb shell dumpsys SurfaceFlinger --display-id`).

These are local test tools in Python, as the repository's other review tools; the app itself is Rust.
