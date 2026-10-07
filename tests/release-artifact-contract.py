"""Guard the artifact-name/layout handoff between native and release jobs."""

from pathlib import Path
import re
import unittest


WORKFLOW = Path(".github/workflows/verify.yml").read_text(encoding="utf-8")


class ReleaseArtifactContractTests(unittest.TestCase):
    def test_uploaded_binary_basename_is_kept_inside_target_named_directory(self):
        self.assertIn("name: awtrix-cli-${{ matrix.target }}", WORKFLOW)
        self.assertIn("merge-multiple: false", WORKFLOW)
        mappings = (
            ("x86_64-unknown-linux-gnu", "awtrix-cli", "awtrix-cli-x86_64-unknown-linux-gnu"),
            ("aarch64-apple-darwin", "awtrix-cli", "awtrix-cli-aarch64-apple-darwin"),
            ("x86_64-pc-windows-msvc", "awtrix-cli.exe", "awtrix-cli-x86_64-pc-windows-msvc.exe"),
        )
        for target, binary, asset in mappings:
            line = (
                f"install -m 0755 artifacts/awtrix-cli-{target}/{binary} "
                f"release-assets/{asset}"
            )
            with self.subTest(target=target):
                self.assertIn(line, WORKFLOW)

    def test_release_job_waits_for_native_checks_and_verifies_assets_first(self):
        release_job = WORKFLOW.split("  release:\n", maxsplit=1)[1]
        self.assertRegex(release_job, re.compile(r"needs:\s*host-test"))
        self.assertIn("gh release upload \"$TAG\" release-assets/* --clobber", release_job)
        self.assertIn("sha256sum awtrix-cli-x86_64-unknown-linux-gnu", release_job)
        self.assertIn("gh release edit \"$TAG\" --draft=false --latest", release_job)


if __name__ == "__main__":
    unittest.main()
