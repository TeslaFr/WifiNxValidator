#!/usr/bin/env python3
"""Validate tagged native builds and produce the five release archives."""

import argparse
import hashlib
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile

TARGETS = {
    "x86_64-pc-windows-msvc": ("windows", "x86_64", ".zip"),
    "x86_64-unknown-linux-gnu": ("linux", "x86_64", ".tar.gz"),
    "aarch64-unknown-linux-gnu": ("linux", "aarch64", ".tar.gz"),
    "x86_64-apple-darwin": ("macos", "x86_64", ".zip"),
    "aarch64-apple-darwin": ("macos", "aarch64", ".zip"),
}
DOCUMENTS = ("README.md", "README-FR.md", "LICENSE", "THIRD_PARTY_NOTICES.md")


def validate_tag(root, tag):
    if not re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag):
        raise ValueError("Release tag must be exactly vX.X.X without leading zeroes")
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if tag[1:] != version:
        raise ValueError(f"Tag {tag} does not match Cargo.toml package.version {version}")
    return version


def archive_name(tag, target):
    platform, arch, extension = TARGETS[target]
    return f"wifinx-validator-{tag}-{platform}-{arch}{extension}"


def validate_host(target):
    details = subprocess.check_output(["rustc", "-vV"], text=True)
    if f"host: {target}" not in details.splitlines():
        raise ValueError(f"This job must build natively on rustc host {target}")


def package(root, output, tag, target):
    version = validate_tag(root, tag)
    validate_host(target)
    platform, _, extension = TARGETS[target]
    executable = "wifinx-validator" + (".exe" if platform == "windows" else "")
    binary = root / "target" / "release" / executable
    if not binary.is_file():
        raise ValueError(f"Missing release executable: {binary}")
    for document in DOCUMENTS:
        if not (root / document).is_file():
            raise ValueError(f"Missing release document: {document}")
    output.mkdir(parents=True, exist_ok=True)
    destination = output / archive_name(tag, target)
    with tempfile.TemporaryDirectory() as temp:
        staging = Path(temp) / destination.name.removesuffix(extension)
        staging.mkdir()
        for document in DOCUMENTS:
            shutil.copy2(root / document, staging / document)
        if platform == "macos":
            app = staging / "WifiNxValidator.app"
            contents = app / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            for language in ("en", "fr"):
                (contents / "Resources" / f"{language}.lproj").mkdir(parents=True)
            shutil.copy2(binary, contents / "MacOS" / "wifinx-validator")
            (contents / "MacOS" / "wifinx-validator").chmod(0o755)
            with (contents / "Info.plist").open("wb") as handle:
                plistlib.dump({
                    "CFBundleExecutable": "wifinx-validator",
                    "CFBundleIdentifier": "org.wifinx.Validator",
                    "CFBundleName": "WifiNxValidator",
                    "CFBundleDisplayName": "WifiNxValidator",
                    "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version,
                    "CFBundleVersion": version,
                    "LSMinimumSystemVersion": "11.0",
                    "NSHighResolutionCapable": True,
                    "NSLocalNetworkUsageDescription": "WifiNx Validator responds to local connection checks from your console.",
                    "CFBundleDevelopmentRegion": "en",
                    "CFBundleLocalizations": ["en", "fr"],
                }, handle)
            descriptions = {
                "en": '"NSLocalNetworkUsageDescription" = "WifiNx Validator responds to local connection checks from your console.";\n',
                "fr": '"NSLocalNetworkUsageDescription" = "WifiNx Validator répond aux tests de connexion locaux de votre console.";\n',
            }
            for language, description in descriptions.items():
                (contents / "Resources" / f"{language}.lproj" / "InfoPlist.strings").write_text(description, encoding="utf-8")
            subprocess.run(["plutil", "-lint", str(contents / "Info.plist")], check=True)
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            subprocess.run(["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)], check=True)
            subprocess.run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(staging), str(destination.resolve())], check=True)
        else:
            shutil.copy2(binary, staging / executable)
            if platform == "linux":
                (staging / executable).chmod(0o755)
                with tarfile.open(destination, "w:gz") as archive:
                    archive.add(staging, arcname=staging.name)
            else:
                with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                    for path in sorted(staging.rglob("*")):
                        archive.write(path, path.relative_to(staging.parent))
    print(destination)


def checksums(output, tag):
    expected = {archive_name(tag, target) for target in TARGETS}
    actual = {path.name for path in output.iterdir() if path.is_file() and path.name != "SHA256SUMS"}
    if actual != expected:
        raise ValueError(f"Release archives differ: missing={expected - actual}, unexpected={actual - expected}")
    lines = []
    for name in sorted(expected):
        with (output / name).open("rb") as handle:
            digest = hashlib.file_digest(handle, "sha256").hexdigest()
        lines.append(f"{digest}  {name}\n")
    (output / "SHA256SUMS").write_text("".join(lines), encoding="ascii")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("validate", "package", "checksums"))
    parser.add_argument("--tag", required=True)
    parser.add_argument("--target", choices=TARGETS)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--output", type=Path, default=Path("dist"))
    args = parser.parse_args()
    validate_tag(args.root, args.tag)
    if args.mode in ("validate", "package"):
        if not args.target:
            parser.error("--target is required for native validation and packaging")
        validate_host(args.target)
    if args.mode == "package":
        package(args.root, args.output, args.tag, args.target)
    elif args.mode == "checksums":
        checksums(args.output, args.tag)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        sys.exit(str(error))
