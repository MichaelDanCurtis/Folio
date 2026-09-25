#!/usr/bin/env python3
"""Fetch the Office-created compatibility corpus.

Word and Excel files saved by Microsoft Office itself, taken from Apache POI's
test data (Apache License 2.0) at a pinned commit. Only files whose
docProps/app.xml names Microsoft Word or Excel as the saving application are
kept. The files go to build/office-corpus (not committed); the manifest with
their hashes is docs/status/office-corpus.tsv.

    python3 scripts/verification/fetch_office_corpus.py           # fetch and verify
    python3 scripts/verification/fetch_office_corpus.py --write   # rewrite the manifest

Then run the checks:

    cd rust && cargo test --release office_corpus -- --ignored --nocapture
"""
import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

POI = "https://github.com/apache/poi.git"
COMMIT = "942d95d85b15d0dfdb3bc9ba1b4f273f277757c8"
DIRS = {"test-data/spreadsheet": ".xlsx", "test-data/document": ".docx"}
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "build" / "office-corpus"
MANIFEST = ROOT / "docs" / "status" / "office-corpus.tsv"
HEADER = [
    "# Office-created compatibility corpus. Fetch with",
    "# scripts/verification/fetch_office_corpus.py; source: Apache POI test data",
    f"# ({POI} at {COMMIT}), Apache License 2.0.",
    "# file\tsha256\tapplication\tapp_version\tbytes",
]


def application(path: Path) -> tuple[str, str]:
    """The saving application and version recorded in docProps/app.xml."""
    try:
        with zipfile.ZipFile(path) as z:
            app = z.read("docProps/app.xml").decode("utf-8", "replace")
    except (zipfile.BadZipFile, KeyError):
        return "", ""
    name = re.search(r"<(?:\w+:)?Application>([^<]*)<", app)
    version = re.search(r"<(?:\w+:)?AppVersion>([^<]*)<", app)
    return (name.group(1) if name else "", version.group(1) if version else "")


def fetch(checkout: Path) -> None:
    subprocess.run(
        ["git", "clone", "--filter=blob:none", "--no-checkout", "--sparse", POI, str(checkout)],
        check=True,
    )
    subprocess.run(["git", "-C", str(checkout), "sparse-checkout", "set", *DIRS], check=True)
    subprocess.run(["git", "-C", str(checkout), "checkout", COMMIT], check=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rewrite the manifest")
    parser.add_argument("--checkout", type=Path, help="use an existing POI checkout")
    args = parser.parse_args()

    with tempfile.TemporaryDirectory() as tmp:
        checkout = args.checkout or Path(tmp) / "poi"
        if not args.checkout:
            fetch(checkout)
        rows = []
        for folder, extension in DIRS.items():
            for path in sorted((checkout / folder).glob(f"*{extension}")):
                app, version = application(path)
                if not re.match(r"Microsoft (Office |Macintosh )?(Word|Excel)", app):
                    continue
                data = path.read_bytes()
                rows.append(
                    [path.name, hashlib.sha256(data).hexdigest(), app, version, str(len(data))]
                )
                OUT.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(path, OUT / path.name)

    if args.write:
        MANIFEST.write_text("\n".join(HEADER + ["\t".join(r) for r in rows]) + "\n")
        print(f"wrote {len(rows)} entries to {MANIFEST}")
        return 0
    expected = {
        line.split("\t")[0]: line.split("\t")[1]
        for line in MANIFEST.read_text().splitlines()
        if line and not line.startswith("#")
    }
    actual = {r[0]: r[1] for r in rows}
    if expected != actual:
        missing = sorted(set(expected) - set(actual))
        changed = sorted(k for k in expected if k in actual and expected[k] != actual[k])
        print(f"manifest mismatch: missing {missing[:10]}, changed {changed[:10]}")
        return 1
    print(f"{len(rows)} files in {OUT}, matching the manifest")
    return 0


if __name__ == "__main__":
    sys.exit(main())
