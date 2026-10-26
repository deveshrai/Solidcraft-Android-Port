"""AppRun puts a `solidcraft-cli` wrapper on PATH (~/.local/bin) and the wrapper runs the bundled CLI."""
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class AppRunCli(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.appdir = self.tmp / "app" / "AppDir"
        (self.appdir / "usr/bin").mkdir(parents=True)
        (self.appdir / "usr/share/applications").mkdir(parents=True)
        shutil.copy(ROOT / "packaging/linux/AppRun", self.appdir / "AppRun")
        (self.appdir / "usr/share/applications/ai.storyteller.solidcraft.desktop").write_text("[Desktop Entry]\nExec=solidcraft\n")
        for name in ("solidcraft", "solidcraft-cli"):
            p = self.appdir / "usr/bin" / name
            p.write_text(f"#!/bin/sh\necho {name} \"$@\"\n")
            p.chmod(0o755)
        # The AppImage file: running it runs AppRun, as the runtime does.
        self.image = self.tmp / "My App" / "SolidCraft.AppImage"
        self.image.parent.mkdir()
        self.image.write_text(f'#!/bin/sh\nexec "{self.appdir}/AppRun" "$@"\n')
        self.image.chmod(0o755)
        self.home = self.tmp / "home"
        self.home.mkdir()
        self.env = {"PATH": os.environ["PATH"], "HOME": str(self.home), "APPIMAGE": str(self.image)}
        self.wrapper = self.home / ".local/bin/solidcraft-cli"

    def run_apprun(self, *args, **extra):
        env = {**self.env, **extra}
        return subprocess.run([str(self.appdir / "AppRun"), *args], env=env, capture_output=True, text=True, check=True).stdout.strip()

    def test_wrapper_runs_the_bundled_cli(self):
        self.assertEqual(self.run_apprun("a"), "solidcraft a")
        self.assertTrue(self.wrapper.is_file())
        out = subprocess.run([str(self.wrapper), "mcp", "setup"], env=self.env, capture_output=True, text=True, check=True).stdout.strip()
        self.assertEqual(out, "solidcraft-cli mcp setup")

    def test_wrapper_follows_a_moved_appimage(self):
        self.run_apprun()
        moved = self.tmp / "moved.AppImage"
        shutil.move(self.image, moved)
        self.env["APPIMAGE"] = str(moved)
        self.run_apprun()
        self.assertIn(str(moved), self.wrapper.read_text())

    def test_a_file_that_is_not_ours_is_left_alone(self):
        self.wrapper.parent.mkdir(parents=True)
        self.wrapper.write_text("#!/bin/sh\necho mine\n")
        self.run_apprun()
        self.assertEqual(self.wrapper.read_text(), "#!/bin/sh\necho mine\n")
        other = self.home / ".local/bin/solidcraft-cli"
        other.unlink()
        other.symlink_to("/bin/true")
        self.run_apprun()
        self.assertTrue(other.is_symlink())

    def test_opt_out_installs_nothing(self):
        self.run_apprun(SOLIDCRAFT_NO_DESKTOP_INTEGRATION="1")
        self.assertFalse(self.wrapper.exists())


if __name__ == "__main__":
    unittest.main()
