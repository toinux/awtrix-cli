"""Offline public-CLI tests for POSIX and PowerShell release installers."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SH_INSTALLER = ROOT / "skills/awtrix-cli/scripts/install.sh"
PS_INSTALLER = ROOT / "skills/awtrix-cli/scripts/install.ps1"
INSTALLATION_GUIDE = ROOT / "skills/awtrix-cli/references/installation.md"
SKILL = ROOT / "skills/awtrix-cli/SKILL.md"


def skill_description():
    lines = SKILL.read_text().splitlines()
    description_start = next(index for index, line in enumerate(lines) if line == "description: >-") + 1
    description_lines = []
    for line in lines[description_start:]:
        if line and not line.startswith(" "):
            break
        description_lines.append(line.strip())
    return " ".join(description_lines)


class SkillInstallationGuidanceTests(unittest.TestCase):
    def test_skill_description_selects_awtrix_cli_information_and_setup_requests(self):
        description = skill_description()

        for intent in (
            "AWTRIX context",
            "awtrix-cli version",
            "availability",
            "location",
            "installation",
            "update",
            "use",
            "troubleshooting",
            "displays",
            "firmware",
            "Berry scripts",
            "modules",
            "resources",
            "awtrix.toml projects",
        ):
            with self.subTest(intent=intent):
                self.assertIn(intent, description, f"Skill selection description should cover {intent!r}")

        self.assertIn("incidental AWTRIX mention", description, "Incidental AWTRIX mentions should be excluded")
        self.assertIn("unrelated generic CLI request", description, "Unrelated generic CLI requests should be excluded")

    def test_guidance_prefers_verified_binaries_and_uses_explicit_update(self):
        guide = INSTALLATION_GUIDE.read_text()
        skill = SKILL.read_text()
        self.assertLess(guide.index("Preferred path: verified GitHub release binary"), guide.index("Source-build fallback: Cargo"))
        self.assertIn("awtrix-cli update", guide)
        self.assertIn("SHA256SUMS", guide)
        self.assertIn("awtrix-cli-x86_64-unknown-linux-gnu", guide)
        self.assertIn("awtrix-cli-aarch64-apple-darwin", guide)
        self.assertIn("awtrix-cli-x86_64-pc-windows-msvc.exe", guide)
        self.assertIn("never installs anything automatically", guide)
        self.assertIn("does not install the CLI", guide)
        self.assertIn("awtrix-cli update", skill)


@unittest.skipIf(os.name == "nt", "POSIX shell behavior test")
class PosixInstallerTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.tools, self.downloads = self.root / "tools", self.root / "downloads"
        self.tools.mkdir()
        self.downloads.mkdir()
        self.home, self.url_log = self.root / "home", self.root / "urls"
        self.home.mkdir()
        self.tool("uname", '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo x86_64;; esac\n')
        self.tool("awtrix-cli", '#!/bin/sh\necho wrong-existing-cli >&2\nexit 1\n')
        self.tool("curl", f'''#!/bin/sh
url=; out=; previous=
for x do
  [ "$previous" = -o ] && out=$x
  case "$x" in https://*) url=$x;; esac
  previous=$x
done
printf '%s\\n' "$url" >> "{self.url_log}"
case "$url" in
 https://github.com/toinux/awtrix-cli/releases/latest) printf '%s' 'https://github.com/toinux/awtrix-cli/releases/tag/v0.1.0'; exit 0 ;;
 */SHA256SUMS) cp "{self.downloads}/SHA256SUMS" "$out"; exit $? ;;
 *releases/download*) [ -f "{self.downloads}/binary" ] || exit 22; cp "{self.downloads}/binary" "$out"; exit $? ;;
esac
exit 0
''')

    def tearDown(self):
        self.temp.cleanup()

    def tool(self, name, content):
        path = self.tools / name
        path.write_text(content)
        path.chmod(0o755)

    def invoke(self, *args, existing=False):
        env = os.environ.copy()
        env.update(PATH=f"{self.tools}:/usr/bin:/bin", HOME=str(self.home), TMPDIR=str(self.root))
        if existing:
            self.tool("awtrix-cli", '#!/bin/sh\ncase "$1" in --version) echo "awtrix-cli 9.9.9";; --help) echo help;; esac\nexit 0\n')
        return subprocess.run(["sh", str(SH_INSTALLER), *args], env=env, text=True, capture_output=True)

    def release(self, valid=True, asset="awtrix-cli-x86_64-unknown-linux-gnu"):
        body = b'#!/bin/sh\ncase "$1" in --version) echo "awtrix-cli 0.1.0";; --help) echo help;; esac\n'
        (self.downloads / "binary").write_bytes(body)
        digest = hashlib.sha256(body).hexdigest() if valid else "0" * 64
        (self.downloads / "SHA256SUMS").write_text(f"{digest}  {asset}\n")

    def test_working_existing_command_skips_downloads_and_propagates_help_failure(self):
        self.assertEqual(self.invoke(existing=True).returncode, 0)
        self.assertFalse(self.url_log.exists())
        self.tool("awtrix-cli", '#!/bin/sh\n[ "$1" = --version ] && echo "awtrix-cli 9.9.9" && exit 0\nexit 41\n')
        failed = self.invoke()
        self.assertEqual(failed.returncode, 41)

    def test_download_failure_keeps_prior_destination(self):
        target = self.root / "installed/awtrix-cli"
        target.parent.mkdir()
        target.write_text("old")
        result = self.invoke("--install-dir", str(target.parent))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(target.read_text(), "old")

    def test_bad_checksum_keeps_prior_destination(self):
        self.release(valid=False)
        target = self.root / "installed/awtrix-cli"
        target.parent.mkdir()
        target.write_text("old")
        result = self.invoke("--version", "v0.1.0", "--install-dir", str(target.parent))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA-256 verification failed", result.stderr)
        self.assertEqual(target.read_text(), "old")

    def test_verified_binary_is_checked_before_replacement_and_installed(self):
        self.release()
        target = self.root / "new-install"
        result = self.invoke("--install-dir", str(target))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(os.access(target / "awtrix-cli", os.X_OK))
        self.assertIn("awtrix-cli 0.1.0", result.stdout)
        self.assertIn("/releases/download/v0.1.0/awtrix-cli-x86_64-unknown-linux-gnu", self.url_log.read_text())

    def test_invalid_and_empty_component_versions_are_rejected(self):
        for version in ("latest", "v1..0.0"):
            with self.subTest(version=version):
                result = self.invoke("--version", version)
                self.assertEqual(result.returncode, 2)

    def test_verified_but_wrong_version_preserves_destination(self):
        self.release()
        binary = self.downloads / "binary"
        binary.write_bytes(binary.read_bytes().replace(b"awtrix-cli 0.1.0", b"awtrix-cli 0.2.0"))
        checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
        (self.downloads / "SHA256SUMS").write_text(f"{checksum}  awtrix-cli-x86_64-unknown-linux-gnu\n")
        target = self.root / "installed/awtrix-cli"
        target.parent.mkdir()
        target.write_text("old")
        result = self.invoke("--version", "v0.1.0", "--install-dir", str(target.parent))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unexpected binary version", result.stderr)
        self.assertEqual(target.read_text(), "old")

    def test_verified_binary_help_failure_preserves_destination(self):
        binary = self.downloads / "binary"
        binary.write_bytes(b'#!/bin/sh\n[ "$1" = --version ] && echo "awtrix-cli 0.1.0" && exit 0\nexit 42\n')
        checksum = hashlib.sha256(binary.read_bytes()).hexdigest()
        (self.downloads / "SHA256SUMS").write_text(f"{checksum}  awtrix-cli-x86_64-unknown-linux-gnu\n")
        target = self.root / "installed/awtrix-cli"
        target.parent.mkdir()
        target.write_text("old")
        result = self.invoke("--version", "v0.1.0", "--install-dir", str(target.parent))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Downloaded binary failed --help", result.stderr)
        self.assertEqual(target.read_text(), "old")

    def test_latest_resolution_pins_tag_and_macos_arm64_asset(self):
        self.tool("uname", '#!/bin/sh\ncase "$1" in -s) echo Darwin;; -m) echo arm64;; esac\n')
        asset = "awtrix-cli-aarch64-apple-darwin"
        self.release(asset=asset)
        result = self.invoke("--install-dir", str(self.root / "mac-bin"))
        self.assertEqual(result.returncode, 0, result.stderr)
        urls = self.url_log.read_text()
        self.assertIn("https://github.com/toinux/awtrix-cli/releases/latest", urls)
        self.assertIn(f"/releases/download/v0.1.0/{asset}", urls)
        self.assertNotIn("/releases/download/latest/", urls)

    def test_unsupported_architecture_fails_before_download(self):
        self.tool("uname", '#!/bin/sh\ncase "$1" in -s) echo Linux;; -m) echo aarch64;; esac\n')
        result = self.invoke()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unsupported host Linux/aarch64", result.stderr)


@unittest.skipUnless(os.name == "nt" and shutil.which("pwsh"), "requires native Windows and PowerShell Core")
class WindowsInstallerTests(unittest.TestCase):
    """Windows tests use the actual release executable supplied by native CI."""

    def setUp(self):
        binary = os.environ.get("AWTRIX_DISTRIBUTION_BINARY")
        if not binary or not Path(binary).is_file():
            self.skipTest("AWTRIX_DISTRIBUTION_BINARY must identify the built Windows release executable")
        self.binary = Path(binary)
        version = subprocess.check_output([str(self.binary), "--version"], text=True).strip()
        self.assertTrue(version.startswith("awtrix-cli "))
        self.tag = "v" + version.split()[1]
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.asset = "awtrix-cli-x86_64-pc-windows-msvc.exe"
        self.fixture = self.root / "fixture.exe"
        shutil.copy2(self.binary, self.fixture)
        self.checksum = hashlib.sha256(self.fixture.read_bytes()).hexdigest()
        self.urls = self.root / "urls.txt"
        self.wrapper = self.root / "run-installer.ps1"
        self.wrapper.write_text(self._wrapper(), encoding="utf-8")

    def tearDown(self):
        if hasattr(self, "temp"):
            self.temp.cleanup()

    def _wrapper(self):
        installer = str(PS_INSTALLER).replace("'", "''")
        fixture = str(self.fixture).replace("'", "''")
        urls = str(self.urls).replace("'", "''")
        return f'''$ErrorActionPreference = 'Stop'
function Invoke-RestMethod {{ [pscustomobject]@{{ tag_name = '{self.tag}' }} }}
function Invoke-WebRequest {{
  param($Uri, $OutFile, $Headers, $TimeoutSec, [switch]$UseBasicParsing)
  Add-Content -LiteralPath '{urls}' -Value $Uri
  if ($OutFile -like '*SHA256SUMS') {{ Set-Content -LiteralPath $OutFile -Value '{self.checksum}  {self.asset}' }}
  else {{ Copy-Item -LiteralPath '{fixture}' -Destination $OutFile -Force }}
}}
& '{installer}' @args
exit $LASTEXITCODE
'''

    def invoke(self, *args):
        return subprocess.run(["pwsh", "-NoProfile", "-File", str(self.wrapper), *args], text=True, capture_output=True)

    def test_fresh_install_resolves_latest_and_pins_downloads(self):
        target = self.root / "fresh-install"
        result = self.invoke("-InstallDir", str(target))
        self.assertEqual(result.returncode, 0, result.stderr)
        installed = target / "awtrix-cli.exe"
        self.assertTrue(installed.is_file())
        self.assertEqual(hashlib.sha256(installed.read_bytes()).hexdigest(), self.checksum)
        self.assertIn(f"/releases/download/{self.tag}/", self.urls.read_text())
        self.assertNotIn("/releases/download/latest/", self.urls.read_text())

    def test_explicit_version_and_bad_checksum_preserve_old_destination(self):
        # Wrapper fixture alteration simulates a mismatching manifest without bypassing verification.
        self.wrapper.write_text(self._wrapper().replace(self.checksum, "0" * 64), encoding="utf-8")
        target = self.root / "bad-checksum"
        target.mkdir()
        destination = target / "awtrix-cli.exe"
        destination.write_bytes(b"old executable")
        result = self.invoke("-Version", self.tag, "-InstallDir", str(target))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(destination.read_bytes(), b"old executable")
        self.assertIn(f"/releases/download/{self.tag}/", self.urls.read_text())
        self.assertNotIn("awtrix-cli " + self.tag[1:], result.stdout)

    def test_existing_binary_fast_path_does_not_download(self):
        existing = self.root / "existing"
        existing.mkdir()
        shutil.copy2(self.binary, existing / "awtrix-cli.exe")
        env = os.environ.copy()
        env["PATH"] = f"{existing}{os.pathsep}{env['PATH']}"
        result = subprocess.run(["pwsh", "-NoProfile", "-File", str(PS_INSTALLER)], env=env, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("already installed", result.stdout)

    def test_unsupported_windows_host_fails_before_release_lookup(self):
        env = os.environ.copy()
        env["OS"] = "NotWindows"
        result = subprocess.run(["pwsh", "-NoProfile", "-File", str(PS_INSTALLER)], env=env, text=True, capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Unsupported host", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
