"""Checks for tag-gated release validation and complete, repeatable checksums."""
import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("release", Path(__file__).parents[1] / "scripts" / "release.py")
release = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def test_only_exact_matching_version_tags_are_accepted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nversion = "0.1.0"\n', encoding="utf-8")
            self.assertEqual(release.validate_tag(root, "v0.1.0"), "0.1.0")
            for invalid in ("master", "0.1.0", "v0.1", "v0.1.0-beta", "v00.1.0", "v0.1.0/extra", "v0.1.1"):
                with self.subTest(tag=invalid), self.assertRaises(ValueError):
                    release.validate_tag(root, invalid)

    def test_five_architectures_produce_distinct_archive_names(self):
        names = {release.archive_name("v0.1.0", target) for target in release.TARGETS}
        self.assertEqual(len(names), 5)
        self.assertTrue(any("linux-aarch64" in name for name in names))
        self.assertTrue(any("macos-x86_64" in name for name in names))
        self.assertTrue(any("macos-aarch64" in name for name in names))

    def test_missing_or_unexpected_archives_block_publication(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with self.assertRaises(ValueError):
                release.checksums(output, "v0.1.0")
            for target in release.TARGETS:
                (output / release.archive_name("v0.1.0", target)).write_bytes(b"archive")
            (output / "unexpected.zip").write_bytes(b"stale artifact")
            with self.assertRaises(ValueError):
                release.checksums(output, "v0.1.0")

    def test_checksums_cover_all_assets_and_can_be_regenerated(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            for target in release.TARGETS:
                name = release.archive_name("v0.1.0", target)
                (output / name).write_bytes(name.encode())
            release.checksums(output, "v0.1.0")
            original = (output / "SHA256SUMS").read_text()
            self.assertEqual(len(original.splitlines()), 5)
            for line in original.splitlines():
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((output / name).read_bytes()).hexdigest())
            release.checksums(output, "v0.1.0")
            self.assertEqual((output / "SHA256SUMS").read_text(), original)


if __name__ == "__main__":
    unittest.main()
