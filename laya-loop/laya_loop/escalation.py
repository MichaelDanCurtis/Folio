"""When should a paragraph go to Claude? Signals from Laya, plus hard loop guards.

Laya is never asked "should we escalate?" directly - small models are weakest at
that kind of self-assessment. Escalation is decided from signals the app has:

- novel:     the sentinel says something is off, but no rule fired -> we lack a rule
- uncertain: a live rule sits inside its uncertainty band -> Laya can't tell

Hits on rules Laya is already confident about never auto-escalate; the user can
still ask Claude about any paragraph manually.
"""

from __future__ import annotations

import threading
import time
from collections import deque
from dataclasses import dataclass, field

from .models import Flag


@dataclass
class GuardConfig:
    max_calls_per_hour: int = 12
    max_pending: int = 5
    paragraph_cooldown_s: float = 600.0   # after escalating a paragraph, leave it alone


def reasons_for(flags: list[Flag]) -> list[str]:
    rules = [f for f in flags if f.kind == "rule"]
    sentinel = [f for f in flags if f.kind == "sentinel"]
    reasons = []
    if any(s.fired for s in sentinel) and not any(r.fired for r in rules):
        reasons.append("novel: the general problem detector fired but no rule covers it")
    for r in rules:
        if r.uncertain and not r.fired:
            reasons.append(f"uncertain: '{r.label}' is borderline ({r.signal:.2f} vs threshold {r.threshold:.2f})")
    return reasons


@dataclass
class EscalationGuard:
    config: GuardConfig = field(default_factory=GuardConfig)
    _sent_text: set[str] = field(default_factory=set)       # text hashes already reviewed
    _paragraph_last: dict[str, float] = field(default_factory=dict)
    _calls: deque = field(default_factory=deque)
    _lock: threading.Lock = field(default_factory=threading.Lock)

    def mark_reviewed(self, text_hash: str) -> None:
        """Text Claude has already seen or written must never auto-escalate again."""
        with self._lock:
            self._sent_text.add(text_hash)

    def blocked(self, text_hash: str, paragraph_id: str, pending: int, now: float | None = None) -> str | None:
        with self._lock:
            return self._blocked(text_hash, paragraph_id, pending, time.time() if now is None else now)

    def _blocked(self, text_hash: str, paragraph_id: str, pending: int, now: float) -> str | None:
        if text_hash in self._sent_text:
            return "already reviewed by Claude"
        last = self._paragraph_last.get(paragraph_id)
        if last is not None and now - last < self.config.paragraph_cooldown_s:
            return "paragraph is cooling down"
        if pending >= self.config.max_pending:
            return "approval queue is full"
        return None

    def budget_left(self, now: float | None = None) -> int:
        now = time.time() if now is None else now
        with self._lock:
            while self._calls and now - self._calls[0] > 3600:
                self._calls.popleft()
            return self.config.max_calls_per_hour - len(self._calls)

    def record_call(self, text_hash: str | None = None, paragraph_id: str | None = None,
                    now: float | None = None) -> None:
        """Count one Claude call. Pass text_hash/paragraph_id only for reviews: that text is then settled."""
        now = time.time() if now is None else now
        with self._lock:
            self._calls.append(now)
            if text_hash:
                self._sent_text.add(text_hash)
            if paragraph_id:
                self._paragraph_last[paragraph_id] = now
