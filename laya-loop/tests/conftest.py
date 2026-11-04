import time

import pytest

from laya_loop.controller import LoopController
from laya_loop.models import CheckSpec, Question
from laya_loop.registry import CheckRegistry


class FakeEngine:
    """Keyword-driven stand-in for Laya. Instructions containing a key get 0.9 on matching text, else 0.05."""

    name = "fake-laya"
    delay = 0.0  # seconds per call; widens race windows in concurrency tests
    KEYWORDS = {
        "hostile": ["stupid", "ignored", "lazy", "embarrassed"],
        "problem": ["stupid", "ignored", "password", "fraud", "guaranteed", "proves"],
        "certain": ["guaranteed", "proves", "definitely", "nobody could"],
        "source": ["according to", "per the", "source:"],
        "deliver": ["by friday", "by march", "by june"],
    }

    def __init__(self):
        self.calls = 0
        self.KEYWORDS = {k: list(v) for k, v in FakeEngine.KEYWORDS.items()}  # per-test copy; tests mutate it
        self.overrides: dict[str, float] = {}  # exact text -> signal for every question

    def signals(self, text: str, questions: dict[str, Question]) -> dict[str, float]:
        self.calls += 1
        if self.delay:
            time.sleep(self.delay)
        out = {}
        low = text.lower()
        for name, q in questions.items():
            instr = q.instructions.lower()
            if text in self.overrides:
                out[name] = self.overrides[text]
                continue
            if "unseparable" in instr:
                out[name] = 0.5
                continue
            keys = [k for key, ks in self.KEYWORDS.items() if key in instr for k in ks]
            out[name] = 0.9 if any(k in low for k in keys) else 0.05
        return out


def spec(id="overclaims_certainty", instructions="Does the text claim something is certain or guaranteed?",
         direction="above", fire=None, not_=None) -> dict:
    return {
        "id": id, "label": f"Flag {id}", "rationale": "", "severity": "medium", "direction": direction,
        "question": {"type": "noul", "instructions": instructions, "criteria": [], "fire_on": []},
        "tests": {
            "should_fire": fire or ["This proves it works.", "Success is guaranteed.", "We will definitely win.",
                                    "Nobody could disagree."],
            "should_not": not_ or ["This suggests it works.", "Success is likely.", "We expect to win.",
                                   "Most people agree."],
        },
    }


class FakeBackend:
    name = "fake"
    label = "Fake Claude"
    simulated = False

    def __init__(self, checks=None, reword=None, applies=True):
        self.checks = checks if checks is not None else [spec()]
        self.reword_result = reword
        self.applies = applies
        self.calls = {"review": 0, "reword": 0, "verify": 0}
        self.last_existing = None

    def review(self, paragraph, context, reasons, existing, rejected, borderline=()):
        self.calls["review"] += 1
        self.last_existing = existing
        self.last_borderline = list(borderline)
        return {"issue_found": True, "issue_summary": "overclaims", "rewrite": "This suggests it may work.",
                "new_checks": self.checks,
                "rule_verdicts": [{"check_id": r["id"], "applies": True, "reason": "fake"} for r in borderline]}

    def reword(self, check, report):
        self.calls["reword"] += 1
        return self.reword_result or {"give_up": True, "note": "", "check": check}

    def verify(self, rule, paragraph):
        self.last_rule = rule
        self.calls["verify"] += 1
        applies = self.applies(paragraph) if callable(self.applies) else self.applies
        return {"applies": applies, "reason": "fake"}


SENTINEL = CheckSpec.model_validate({
    "id": "general_problem", "label": "General problem",
    "question": {"type": "noul", "instructions": "Does this paragraph contain a problem?"},
    "tests": {"should_fire": ["You are stupid.", "The password is x.", "Their CEO is a fraud."],
              "should_not": ["Thanks for the notes.", "See you Tuesday.", "The report is attached."]},
})


@pytest.fixture
def engine():
    return FakeEngine()


@pytest.fixture
def make_controller(engine, tmp_path):
    def _make(backend=None, **kw):
        c = LoopController(engine, CheckRegistry(tmp_path / "rules.json"), backend, **kw)
        c.bootstrap([], SENTINEL)
        return c
    return _make


def wait_done(controller, item_id, timeout=5.0):
    end = time.time() + timeout
    while time.time() < end:
        item = controller._item(item_id)
        if item.state in ("done", "failed"):
            return item
        time.sleep(0.01)
    raise AssertionError(f"{item_id} did not finish")
