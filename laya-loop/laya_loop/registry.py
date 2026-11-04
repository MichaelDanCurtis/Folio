"""Persistent check library with a lifecycle: probation -> active -> retired (or rejected)."""

from __future__ import annotations

import json
import os
import threading
import time
from pathlib import Path

from .models import Check, Status, Verification

WINDOW = 5               # lifecycle decisions look at the most recent verdicts, so a refit rule can recover
PROMOTE_AFTER = 3        # verdicts needed before a probation check can be promoted
PROMOTE_RATE = 0.75
RETIRE_AFTER = 4         # verdicts needed before a probation check can be retired
RETIRE_RATE = 0.5        # retire when Laya matches Claude less often than this
MAX_VERIFICATIONS = 8    # per probation check; bounds Claude spend


class RegistryFull(Exception):
    pass


class CheckRegistry:
    def __init__(self, path: Path | None, max_live: int = 25):
        self.path = path
        self.max_live = max_live
        self._lock = threading.RLock()
        self._checks: dict[str, Check] = {}
        if path and path.exists():
            data = json.loads(path.read_text())
            self._checks = {c["spec"]["id"]: Check.model_validate(c) for c in data["checks"]}

    # -- queries -----------------------------------------------------------
    def all(self) -> list[Check]:
        with self._lock:
            return list(self._checks.values())

    def get(self, check_id: str) -> Check | None:
        with self._lock:
            return self._checks.get(check_id)

    def live(self) -> list[Check]:
        with self._lock:
            return [c for c in self._checks.values() if c.live]

    def unique_id(self, wanted: str) -> str:
        with self._lock:
            if wanted not in self._checks:
                return wanted
            n = 2
            while f"{wanted}_{n}" in self._checks:
                n += 1
            return f"{wanted}_{n}"

    # -- mutations ---------------------------------------------------------
    def add(self, check: Check) -> Check:
        with self._lock:
            if check.id in self._checks:
                raise ValueError(f"check {check.id} already exists")
            if check.live and len(self.live()) >= self.max_live:
                raise RegistryFull(f"rule cap of {self.max_live} live checks reached")
            check.history.append(f"{_now()} {check.status} ({check.origin})")
            self._checks[check.id] = check
            self._save()
            return check

    def add_unique(self, check: Check) -> Check:
        """Add under a fresh id picked inside the lock, so concurrent reviews can't collide."""
        with self._lock:
            check.spec.id = self.unique_id(check.spec.id)
            return self.add(check)

    def set_status(self, check_id: str, status: Status, note: str) -> Check:
        with self._lock:
            check = self._checks[check_id]
            check.status = status
            check.history.append(f"{_now()} {status}: {note}")
            self._save()
            return check

    def set_enabled(self, check_id: str, enabled: bool) -> Check:
        with self._lock:
            check = self._checks[check_id]
            if enabled and not check.enabled and check.status in ("probation", "active") \
                    and len(self.live()) >= self.max_live:
                raise RegistryFull(f"rule cap of {self.max_live} live checks reached")
            check.enabled = enabled
            check.history.append(f"{_now()} {'enabled' if enabled else 'disabled'} by user")
            self._save()
            return check

    def record_fire(self, check_id: str) -> None:
        with self._lock:
            self._checks[check_id].fires += 1
            self._save()

    def add_verification(self, check_id: str, v: Verification) -> tuple[Check, str | None]:
        """Record Claude's verdict on a probation hit; returns (check, lifecycle note or None)."""
        with self._lock:
            check = self._checks[check_id]
            check.verifications.append(v)
            del check.verifications[:-30]  # keep the most recent verdicts as calibration data
            note = None
            if check.status == "probation":
                agree, total = check.agreement(last=WINDOW)
                seen = len(check.verifications)
                confirmed_hit = any(x.fired and x.applies for x in check.verifications)
                if seen >= PROMOTE_AFTER and agree / total >= PROMOTE_RATE and confirmed_hit:
                    note = f"promoted: Laya matched Claude on {agree} of the last {total} real paragraphs"
                    check.status = "active"
                elif seen >= RETIRE_AFTER and agree / total < RETIRE_RATE:
                    note = f"retired: Laya matched Claude on only {agree} of the last {total} real paragraphs"
                    check.status = "retired"
                elif seen >= MAX_VERIFICATIONS:
                    note = f"retired: still undecided after {seen} verdicts ({agree} of the last {total} matched)"
                    check.status = "retired"
                if note:
                    check.history.append(f"{_now()} {note}")
            self._save()
            return check, note

    def update_calibration(self, check_id: str, calibration) -> None:
        with self._lock:
            self._checks[check_id].calibration = calibration
            self._checks[check_id].history.append(
                f"{_now()} recalibrated: threshold {calibration.threshold:.3f} ({calibration.reason})")
            self._save()

    def needs_verification(self, check_id: str) -> bool:
        with self._lock:
            c = self._checks.get(check_id)
            return bool(c and c.status == "probation" and len(c.verifications) < MAX_VERIFICATIONS)

    def _save(self) -> None:
        if not self.path:
            return
        tmp = self.path.with_suffix(".tmp")
        tmp.write_text(json.dumps({"checks": [c.model_dump() for c in self._checks.values()]}, indent=1))
        os.replace(tmp, self.path)


def _now() -> str:
    return time.strftime("%H:%M:%S")
