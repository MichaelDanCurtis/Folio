"""The two-way loop: Laya flags -> user approves -> Claude fixes and writes checks -> Laya tests them -> probation."""

from __future__ import annotations

import itertools
import logging
import threading
import time
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from dataclasses import asdict, dataclass, field
from typing import Literal

from pydantic import ValidationError

from .claude import ClaudeError
from .engine import Engine, text_key
from .escalation import EscalationGuard, reasons_for
from .models import Check, CheckSpec, Flag, Verification
from .registry import MAX_VERIFICATIONS, CheckRegistry, RegistryFull
from .validator import validate

log = logging.getLogger("laya_loop")

MIN_PARAGRAPH_CHARS = 12
MAX_NEW_CHECKS = 2
_ids = itertools.count(1)


@dataclass
class QueueItem:
    kind: Literal["review", "verify"]
    text: str
    reasons: list[str]
    paragraph_id: str | None = None
    check_id: str | None = None
    borderline: list[str] = field(default_factory=list)  # rules Laya was unsure about on this paragraph
    context: str = ""
    manual: bool = False
    state: Literal["pending", "running", "done", "failed", "dismissed"] = "pending"
    error: str | None = None
    result: str | None = None
    id: str = field(default_factory=lambda: f"q{next(_ids)}")
    created_at: float = field(default_factory=time.time)

    @property
    def text_hash(self) -> str:
        return text_key(self.text)


@dataclass
class Proposal:
    check_id: str
    label: str
    status: str
    reason: str
    reworded: bool = False


@dataclass
class Suggestion:
    paragraph_id: str | None
    original: str
    issue_found: bool
    summary: str
    rewrite: str | None
    proposals: list[Proposal]
    backend: str
    state: Literal["open", "accepted", "dismissed"] = "open"
    id: str = field(default_factory=lambda: f"s{next(_ids)}")
    created_at: float = field(default_factory=time.time)


def _to_spec(raw: dict) -> CheckSpec:
    raw = dict(raw)
    q = dict(raw.get("question", {}))
    q["criteria"] = q.get("criteria") or None
    q["fire_on"] = q.get("fire_on") or None
    raw["question"] = q
    return CheckSpec.model_validate(raw)


def _errors(e: ValidationError) -> str:
    return "; ".join(f"{'.'.join(map(str, err['loc']))}: {err['msg']}" for err in e.errors()[:3])


class LoopController:
    def __init__(self, engine: Engine, registry: CheckRegistry, backend=None,
                 guard: EscalationGuard | None = None, auto_verify: bool = False, workers: int = 2):
        self.engine = engine
        self.registry = registry
        self.backend = backend
        self.guard = guard or EscalationGuard()
        self.auto_verify = auto_verify
        self.rules_version = 0
        self._lock = threading.RLock()
        self._queue: list[QueueItem] = []
        self._suggestions: list[Suggestion] = []
        self._events: deque[dict] = deque(maxlen=80)
        self._fire_seen: set[tuple[str, str]] = set()
        self._verify_seen: set[tuple[str, str]] = set()
        self._doc: list[dict] = []
        self._pool = ThreadPoolExecutor(max_workers=workers, thread_name_prefix="claude")

    # -- setup ---------------------------------------------------------------
    def bootstrap(self, rules: list[CheckSpec], sentinel: CheckSpec | None) -> None:
        for spec, kind in [*((r, "rule") for r in rules), *([(sentinel, "sentinel")] if sentinel else [])]:
            if self.registry.get(spec.id):
                continue
            cal = validate(spec, self.engine)
            status = "active" if cal.passed else "rejected"
            self.registry.add(Check(spec=spec, kind=kind, origin="seed", status=status, calibration=cal))
            self._event(f"seed check '{spec.id}' {status}: {cal.reason}")

    # -- the fast path ---------------------------------------------------------
    def analyze(self, paragraphs: list[dict]) -> dict:
        start = time.perf_counter()
        fresh_before = getattr(self.engine, "fresh", 0)
        live = self.registry.live()
        questions = {c.id: c.spec.question for c in live}
        results: dict[str, dict] = {}
        with self._lock:
            self._doc = [{"id": p["id"], "text": p["text"]} for p in paragraphs]
            # Pending approvals track the live document: once a paragraph is edited or deleted, its old text
            # is no longer worth sending (the new text gets evaluated on its own merits).
            current = {text_key(p["text"].strip()) for p in paragraphs}
            for q in self._queue:
                if q.state == "pending" and q.text_hash not in current:
                    q.state, q.result = "dismissed", "paragraph changed before it was sent"
        for i, p in enumerate(paragraphs):
            text = p["text"].strip()
            if len(text) < MIN_PARAGRAPH_CHARS or not questions:
                results[p["id"]] = {"flags": [], "reasons": []}
                continue
            signals = self.engine.signals(text, questions)
            flags = [self._flag(c, signals[c.id]) for c in live]
            th = text_key(text)
            for c, f in zip(live, flags):
                with self._lock:
                    first_fire = f.fired and (th, c.id) not in self._fire_seen
                    if first_fire:
                        self._fire_seen.add((th, c.id))
                if first_fire:
                    self.registry.record_fire(c.id)
                if f.fired and c.status == "probation" and c.kind == "rule":
                    self._enqueue_verify(c, p["id"], text)
            reasons = reasons_for(flags)
            if reasons:
                borderline = [f.check_id for f in flags if f.kind == "rule" and f.uncertain and not f.fired]
                self._enqueue_review(p["id"], text, reasons, _context(paragraphs, i), borderline)
            results[p["id"]] = {"flags": [f.model_dump() for f in flags], "reasons": reasons}
        ms = (time.perf_counter() - start) * 1000
        return {"results": results, "ms": round(ms, 1), "checks": len(live), "rules_version": self.rules_version,
                "fresh": getattr(self.engine, "fresh", 0) - fresh_before}

    @staticmethod
    def _flag(c: Check, signal: float) -> Flag:
        cal = c.calibration  # one snapshot: a concurrent refit replaces this object
        o = c.oriented(signal)
        raw_t = cal.threshold if c.spec.direction == "above" else 1.0 - cal.threshold
        return Flag(check_id=c.id, label=c.spec.label, kind=c.kind, status=c.status, severity=c.spec.severity,
                    signal=round(signal, 4), threshold=round(raw_t, 4), direction=c.spec.direction,
                    fired=o >= cal.threshold, uncertain=abs(o - cal.threshold) < cal.band)

    def _current_context(self, pid: str | None) -> str:
        with self._lock:
            doc = list(self._doc)
        index = next((i for i, p in enumerate(doc) if p["id"] == pid), None)
        return _context(doc, index) if index is not None else ""

    # -- queue -----------------------------------------------------------------
    def _open(self) -> list[QueueItem]:
        return [q for q in self._queue if q.state in ("pending", "running")]

    def _enqueue_review(self, pid: str, text: str, reasons: list[str], context: str,
                        borderline: list[str] = ()) -> None:
        with self._lock:
            th = text_key(text)
            if any(q.kind == "review" and q.text_hash == th for q in self._open()):
                return
            pending_reviews = sum(q.kind == "review" and q.state == "pending" for q in self._queue)
            if self.guard.blocked(th, pid, pending=pending_reviews):
                return
            self._queue.append(QueueItem(kind="review", text=text, reasons=reasons, paragraph_id=pid,
                                         context=context, borderline=list(borderline)))
            self._event(f"paragraph queued for Claude: {reasons[0]}")

    def _enqueue_verify(self, check: Check, pid: str, text: str) -> None:
        with self._lock:
            key = (check.id, text_key(text))
            if key in self._verify_seen or not self.registry.needs_verification(check.id):
                return
            open_for_check = sum(1 for q in self._open() if q.kind == "verify" and q.check_id == check.id)
            if len(check.verifications) + open_for_check >= MAX_VERIFICATIONS:
                return
            self._verify_seen.add(key)
            item = QueueItem(kind="verify", text=text, check_id=check.id, paragraph_id=pid,
                             reasons=[f"probation check '{check.spec.label}' fired; ask Claude if it really applies"])
            self._queue.append(item)
        if self.auto_verify:
            self.approve(item.id)

    def ask(self, pid: str | None, text: str) -> QueueItem:
        """User explicitly asked Claude about a paragraph: skips the guards but still counts against budget."""
        item = QueueItem(kind="review", text=text.strip(), reasons=["the user asked Claude to review this paragraph"],
                         paragraph_id=pid, context=self._current_context(pid), manual=True)
        with self._lock:
            self._queue.append(item)
        self.approve(item.id)
        return item

    def approve(self, item_id: str) -> QueueItem:
        with self._lock:
            item = self._item(item_id)
            if item.state != "pending":
                return item
            if self.backend is None:
                item.state, item.error = "failed", "no Claude backend configured"
                return item
            if not item.manual and self.guard.budget_left() <= 0:
                item.state, item.error = "failed", "hourly Claude budget used up"
                return item
            item.state = "running"
            if item.kind == "review":
                # Neighbours may have been edited since queueing (e.g. a password removed): send what's there now.
                item.context = self._current_context(item.paragraph_id) if item.paragraph_id else item.context
                self.guard.record_call(item.text_hash, item.paragraph_id)
            else:
                self.guard.record_call()
        self._pool.submit(self._run, item)
        return item

    def dismiss(self, item_id: str) -> QueueItem:
        with self._lock:
            item = self._item(item_id)
            if item.state == "pending":
                item.state = "dismissed"
                if item.kind == "review":
                    self.guard.mark_reviewed(item.text_hash)  # don't ask about this text again
            return item

    def _item(self, item_id: str) -> QueueItem:
        for q in self._queue:
            if q.id == item_id:
                return q
        raise KeyError(item_id)

    # -- the slow path -----------------------------------------------------------
    def _run(self, item: QueueItem) -> None:
        try:
            if item.kind == "review":
                self._run_review(item)
            else:
                self._run_verify(item)
            item.state = "done"
        except ClaudeError as e:
            item.state, item.error = "failed", str(e)
            self._event(f"Claude call failed: {e}")
        except Exception as e:  # keep the worker alive; surface the error in the UI
            log.exception("loop step failed")
            item.state, item.error = "failed", f"internal error: {e}"
            self._event(f"internal error: {e}")

    def _run_review(self, item: QueueItem) -> None:
        checks = self.registry.all()
        existing = [{"id": c.id, "label": c.spec.label, "question": c.spec.question.instructions, "status": c.status}
                    for c in checks if c.status in ("active", "probation")]
        rejected = [{"id": c.id, "label": c.spec.label, "why": c.calibration.reason}
                    for c in checks if c.status in ("rejected", "retired")]
        borderline = [c.describe() for c in checks if c.id in item.borderline]
        out = self.backend.review(item.text, item.context, item.reasons, existing, rejected, borderline)
        # Claude's call on each rule Laya was unsure about becomes a labeled example for that rule.
        for verdict in out.get("rule_verdicts", []):
            check = self.registry.get(verdict.get("check_id", ""))
            if check and check.id in item.borderline:
                self._apply_verdict(check, item.text, bool(verdict["applies"]), verdict.get("reason", ""))

        proposals = []
        for raw in out.get("new_checks", [])[:MAX_NEW_CHECKS]:
            try:
                proposals.append(self._admit(raw, item.text_hash))
            except Exception as e:  # one bad proposal must not discard the review
                log.exception("admitting a proposed check failed")
                proposals.append(Proposal(check_id=str(raw.get("id", "?")), label=str(raw.get("label", "?")),
                                          status="rejected", reason=f"internal error: {e}"))
        rewrite = (out.get("rewrite") or "").strip() or None
        if rewrite:
            self.guard.mark_reviewed(text_key(rewrite))  # an accepted rewrite must not bounce back to Claude
        s = Suggestion(paragraph_id=item.paragraph_id, original=item.text, issue_found=bool(out.get("issue_found")),
                       summary=out.get("issue_summary", ""), rewrite=rewrite, proposals=proposals,
                       backend=self.backend.label)
        with self._lock:
            self._suggestions.append(s)
        item.result = s.summary
        self._event(f"Claude reviewed a paragraph: {s.summary or 'no issue'}"
                    + (f" ({len(proposals)} new check(s) proposed)" if proposals else ""))

    def _admit(self, raw: dict, source_hash: str) -> Proposal:
        """Validate a Claude-authored check with Laya; one reword round if it fails and budget allows."""
        label = str(raw.get("label", raw.get("id", "?")))
        try:
            spec = _to_spec(raw)
        except ValidationError as e:
            self._event(f"proposed check '{raw.get('id')}' was malformed: {_errors(e)}")
            return Proposal(check_id=str(raw.get("id", "?")), label=label, status="rejected",
                            reason=f"malformed: {_errors(e)}")
        cal = validate(spec, self.engine)
        reworded = False
        if not cal.passed and self.guard.budget_left() <= 0:
            self._event(f"check '{spec.id}' failed Laya's tests ({cal.reason}); no budget left to reword it")
        elif not cal.passed:
            self._event(f"check '{spec.id}' failed Laya's tests ({cal.reason}); asking Claude to reword")
            self.guard.record_call()
            try:
                rw = self.backend.reword(spec.model_dump(), _report(spec, cal))
                if not rw.get("give_up"):
                    new_spec = _to_spec(rw["check"])
                    new_spec.id = spec.id
                    new_cal = validate(new_spec, self.engine)
                    spec, cal, reworded = new_spec, new_cal, True
            except (ClaudeError, ValidationError, KeyError) as e:
                self._event(f"reword of '{spec.id}' failed: {e}")
        status = "probation" if cal.passed else "rejected"
        check = Check(spec=spec, origin="claude", status=status, calibration=cal)
        with self._lock:
            try:
                self.registry.add_unique(check)
            except RegistryFull as e:
                check.status, cal.reason = "rejected", str(e)
                self.registry.add_unique(check)
            # Claude wrote this check *because of* the source paragraph; verifying it there is wasted spend.
            # Recorded before analyze() can see the new check (it takes this same lock).
            self._verify_seen.add((check.id, source_hash))
        if check.status == "probation":
            self._bump_rules()
        self._event(f"check '{spec.id}' {check.status}{' after reword' if reworded else ''}: {check.calibration.reason}")
        return Proposal(check_id=spec.id, label=spec.label, status=check.status,
                        reason=check.calibration.reason, reworded=reworded)

    def _run_verify(self, item: QueueItem) -> None:
        check = self.registry.get(item.check_id)
        if not check or check.status != "probation":
            item.result = "skipped: check is no longer on probation"
            return
        out = self.backend.verify(check.describe(), item.text)
        applies, reason = bool(out["applies"]), out.get("reason", "")
        item.result = f"{'says the rule applies' if applies else 'says the rule does not apply'}: {reason}"
        self._apply_verdict(check, item.text, applies, reason)

    def _apply_verdict(self, check: Check, text: str, applies: bool, reason: str) -> None:
        """Record Claude's judgment of whether a rule applies to real text, then refit the rule."""
        signal = self.engine.signals(text, {"q": check.spec.question})["q"]
        fired = check.fires_on(signal)
        with self._lock:
            self._verify_seen.add((check.id, text_key(text)))  # Claude has judged this text for this rule
        v = Verification(text=text, signal=signal, applies=applies, reason=reason, fired=fired)
        check, note = self.registry.add_verification(check.id, v)
        verdict = "agreed with" if applies == fired else "corrected"
        self._event(f"Claude {verdict} '{check.id}' ({'applies' if applies else 'does not apply'}): {reason}")

        # Verified real paragraphs become extra test examples; refit the threshold if it still separates.
        pos = [x.text for x in check.verifications if x.applies]
        neg = [x.text for x in check.verifications if not x.applies]
        cal = validate(check.spec, self.engine, extra_pos=pos, extra_neg=neg)
        if cal.passed and abs(cal.threshold - check.calibration.threshold) > 1e-4:
            self.registry.update_calibration(check.id, cal)
            self._bump_rules()
        if note:
            self._bump_rules()
            self._event(f"check '{check.id}' {note}")

    # -- user actions on rules and suggestions -----------------------------------
    def set_enabled(self, check_id: str, enabled: bool) -> None:
        """Raises RegistryFull when enabling would exceed the live-check cap."""
        self.registry.set_enabled(check_id, enabled)
        self._bump_rules()

    def retire(self, check_id: str) -> None:
        self.registry.set_status(check_id, "retired", "retired by user")
        self._bump_rules()

    def resolve_suggestion(self, sid: str, accepted: bool) -> Suggestion:
        with self._lock:
            s = next(x for x in self._suggestions if x.id == sid)
            if s.state == "open":
                s.state = "accepted" if accepted else "dismissed"
            return s

    def _bump_rules(self) -> None:
        with self._lock:
            self.rules_version += 1

    def _event(self, msg: str) -> None:
        with self._lock:
            self._events.append({"at": time.time(), "msg": msg})

    # -- UI snapshot ------------------------------------------------------------
    def state(self) -> dict:
        with self._lock:
            open_items = [q for q in self._queue if q.state in ("pending", "running")]
            finished = [q for q in self._queue if q.state not in ("pending", "running")][-30:]
            self._queue = sorted(open_items + finished, key=lambda q: q.created_at)
            queue = [{**asdict(q), "text_hash": q.text_hash} for q in self._queue]
            suggestions = [asdict(s) for s in self._suggestions[-20:]]
            events = list(self._events)
        return {
            "backend": self.backend.label if self.backend else None,
            "simulated": bool(self.backend and getattr(self.backend, "simulated", False)),
            "engine": self.engine.name,
            "budget_left": self.guard.budget_left(),
            "auto_verify": self.auto_verify,
            "rules_version": self.rules_version,
            "checks": [_check_view(c) for c in self.registry.all()],
            "queue": queue,
            "suggestions": suggestions,
            "events": events,
        }


def _context(doc: list[dict], index: int) -> str:
    parts = []
    if index > 0:
        parts.append(f"Previous paragraph: {doc[index - 1]['text']}")
    if index + 1 < len(doc):
        parts.append(f"Next paragraph: {doc[index + 1]['text']}")
    return "\n".join(parts)


def _report(spec: CheckSpec, cal) -> dict:
    return {
        "result": cal.reason,
        "direction": spec.direction,
        "learned_threshold_oriented": cal.threshold,
        "should_fire": [{"text": t, "signal": s} for t, s in zip(spec.tests.should_fire, cal.pos_signals)],
        "should_not": [{"text": t, "signal": s} for t, s in zip(spec.tests.should_not, cal.neg_signals)],
    }


def _check_view(c: Check) -> dict:
    agree, total = c.agreement()
    cal = c.calibration
    return {
        "id": c.id, "label": c.spec.label, "rationale": c.spec.rationale, "kind": c.kind, "origin": c.origin,
        "status": c.status, "enabled": c.enabled, "severity": c.spec.severity, "direction": c.spec.direction,
        "question": c.spec.question.model_dump(), "threshold": round(c.raw_threshold(), 4),
        "threshold_oriented": cal.threshold,
        "accuracy": cal.accuracy, "margin": cal.margin, "reason": cal.reason, "fires": c.fires,
        "agree": agree, "verified": total,
        "tests": {
            "should_fire": [{"text": t, "signal": s} for t, s in zip(c.spec.tests.should_fire, cal.pos_signals)],
            "should_not": [{"text": t, "signal": s} for t, s in zip(c.spec.tests.should_not, cal.neg_signals)],
        },
        "verifications": [{"text": v.text, "applies": v.applies, "fired": v.fired, "reason": v.reason}
                          for v in c.verifications],
        "history": c.history[-8:],
    }
