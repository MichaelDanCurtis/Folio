#!/usr/bin/env python3
"""Exercise production lifecycle recovery with synthetic notifications, no UI."""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="folio-lifecycle-check-") as temporary:
    binary = Path(temporary) / "verify"
    subprocess.run([
        "xcrun", "swiftc",
        str(root / "macos/Runner/FolioActivationRecovery.swift"),
        str(root / "scripts/verification/verify_activation_recovery.swift"),
        "-o", str(binary),
    ], check=True)
    subprocess.run([str(binary)], check=True)

with tempfile.TemporaryDirectory(prefix="folio-termination-check-") as temporary:
    binary = Path(temporary) / "verify"
    subprocess.run([
        "xcrun", "swiftc",
        str(root / "macos/Runner/FolioTermination.swift"),
        str(root / "scripts/verification/verify_termination.swift"),
        "-o", str(binary),
    ], check=True)
    subprocess.run([str(binary)], check=True)

with tempfile.TemporaryDirectory(prefix="folio-window-frame-check-") as temporary:
    binary = Path(temporary) / "verify"
    subprocess.run([
        "xcrun", "swiftc",
        str(root / "macos/Runner/FolioWindowFrame.swift"),
        str(root / "scripts/verification/verify_window_frame.swift"),
        "-o", str(binary),
    ], check=True)
    subprocess.run([str(binary)], check=True)
