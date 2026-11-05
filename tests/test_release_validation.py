"""Exercise the actual orchestration script against a fake GitHub CLI boundary."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class ReleaseValidationTests(unittest.TestCase):
    def run_validation(self, failure=""):
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "gh"
            fake.write_text("""#!/bin/sh
case "$1 $2" in
  'workflow run') exit 0 ;;
  'run list') echo 123 ;;
  'run watch') if [ "$FAILURE" = checks ]; then exit 1; fi ;;
  'run view')
    case "$*" in
      *headSha*)
        if [ "$FAILURE" = sha ]; then echo 'wrong completed success';
        else echo 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa completed success'; fi ;;
      *) echo 'https://github.example.test/validation/123' ;;
    esac ;;
  *) exit 99 ;;
esac
""")
            fake.chmod(0o700)
            return subprocess.run(
                ["bash", "scripts/validate-release-pr.sh", "release/v0.1.1-42-1", "a" * 40],
                env={**os.environ, "PATH": f"{directory}:{os.environ['PATH']}", "FAILURE": failure},
                capture_output=True, text=True, timeout=10,
            )

    def test_waits_for_success_and_reports_evidence(self):
        result = self.run_validation()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "https://github.example.test/validation/123")

    def test_failed_checks_block_preparation(self):
        result = self.run_validation("checks")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")

    def test_wrong_commit_blocks_preparation(self):
        result = self.run_validation("sha")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
