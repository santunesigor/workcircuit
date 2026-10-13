#!/usr/bin/env python3
"""Package an already-built Linux x86_64 release with dependency notices."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def output(*command):
    return subprocess.check_output(command, cwd=ROOT, text=True).strip()


def main():
    host = next(
        line.removeprefix("host: ")
        for line in output("rustc", "-vV").splitlines()
        if line.startswith("host: ")
    )
    if host != "x86_64-unknown-linux-gnu":
        raise SystemExit("This release package supports native Linux x86_64 builds only.")
    metadata = json.loads(
        output("cargo", "metadata", "--locked", "--format-version=1", "--filter-platform", host)
    )
    project = next(p for p in metadata["packages"] if p["id"] == metadata["resolve"]["root"])
    binary = Path(metadata["target_directory"]) / "release/workcircuit"
    if not binary.is_file():
        raise SystemExit("Run cargo build --release --locked before packaging.")
    name = f"workcircuit-{project['version']}-linux-x86_64"
    destination = ROOT / "dist"
    destination.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="workcircuit-package-") as temporary:
        release = Path(temporary) / name
        release.mkdir()
        shutil.copy2(binary, release / "workcircuit")
        for filename in ("README.md", "CHANGELOG.md"):
            shutil.copy2(ROOT / filename, release / filename)
        shutil.copy2(ROOT / "scripts/install.sh", release / "install.sh")
        (release / "install.sh").chmod(0o755)
        (release / "data").mkdir()
        for asset in (ROOT / "data").iterdir():
            if asset.suffix in (".desktop", ".svg"):
                shutil.copy2(asset, release / "data" / asset.name)

        notices = [
            "# Third-party notices\n",
            "These notices apply to dependencies, not to WorkCircuit's own code. "
            "No license is granted for WorkCircuit at this time.\n",
            "The following inventory includes resolved Rust dependencies and build tools. "
            "System GTK, libadwaita and other shared libraries are supplied by your distribution.\n",
            "| Component | SPDX license expression |\n| --- | --- |",
        ]
        resolved = {node["id"] for node in metadata["resolve"]["nodes"]}
        for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
            if package["id"] not in resolved or package["id"] == project["id"]:
                continue
            source = Path(package["manifest_path"]).parent
            label = f"{package['name']}-{package['version']}"
            licenses = {
                p for p in source.iterdir()
                if p.is_file() and p.name.upper().startswith(
                    ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE")
                )
            }
            if package.get("license_file"):
                licenses.add(source / package["license_file"])
            target = release / "LICENSES" / label
            target.mkdir(parents=True)
            for license_path in sorted(licenses):
                shutil.copy2(license_path, target / license_path.name)
            if (source / "LICENSES").is_dir():
                shutil.copytree(source / "LICENSES", target / "LICENSES")
            if not any(target.iterdir()):
                raise SystemExit(f"Missing license notices for {label}")
            if package["name"] == "libsqlite3-sys":
                sqlite = (source / "sqlite3/sqlite3.c").read_text()
                start = sqlite.index("** 2001 September 15")
                end = sqlite.index("*************************************************************************", start)
                (target / "SQLite-NOTICE.txt").write_text(sqlite[start:end])
            notices.append(f"| {label} | {package.get('license') or 'See included notices'} |")

        rust_notice = Path(output("rustc", "--print", "sysroot")) / "share/doc/rust/COPYRIGHT-library.html"
        if not rust_notice.is_file():
            raise SystemExit("Run rustup component add rust-docs to include Rust library notices.")
        (release / "LICENSES/rust").mkdir()
        shutil.copy2(rust_notice, release / "LICENSES/rust" / rust_notice.name)
        notices.append(f"\nRust standard library: {output('rustc', '--version')}. See LICENSES/rust/.\n")
        (release / "THIRD-PARTY-NOTICES.md").write_text("\n".join(notices))
        archive = destination / f"{name}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            bundle.add(release, arcname=name)
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    (destination / "SHA256SUMS").write_text(f"{digest}  {archive.name}\n")
    print(archive.relative_to(ROOT))


if __name__ == "__main__":
    main()
