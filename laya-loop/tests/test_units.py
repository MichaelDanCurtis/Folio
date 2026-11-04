import pytest
from pydantic import ValidationError

from laya_loop.engine import signal_from_answer
from laya_loop.escalation import EscalationGuard, GuardConfig, reasons_for
from laya_loop.models import Calibration, Check, CheckSpec, Flag, Question, Verification
from laya_loop.registry import CheckRegistry, RegistryFull
from laya_loop.validator import calibrate, validate

from .conftest import spec


# -- calibration ---------------------------------------------------------------
def test_learns_a_low_threshold_for_uncalibrated_but_separable_signals():
    # Measured shape of Laya's hostile-tone check: positives far below 0.5, negatives near 0.
    cal = calibrate([0.48, 0.26, 0.44], [0.01, 0.05, 0.03])
    assert cal.passed
    assert 0.05 < cal.threshold < 0.26
    assert cal.accuracy == 1.0


def test_rejects_examples_laya_cannot_separate():
    # Measured shape of "statistic without a source": sourced and unsourced both ~0.62.
    cal = calibrate([0.62, 0.60, 0.61], [0.63, 0.60, 0.62])
    assert not cal.passed


def test_rejects_perfect_but_tiny_separation():
    cal = calibrate([0.64, 0.65, 0.66], [0.60, 0.61, 0.62])
    assert cal.accuracy == 1.0
    assert not cal.passed and "margin" in cal.reason


def test_needs_three_examples_per_side():
    assert not calibrate([0.9, 0.9], [0.1, 0.1, 0.1]).passed


def test_direction_below_fires_on_absence(engine):
    s = CheckSpec.model_validate(spec(
        id="names_source", instructions="Does the text name a source for its numbers?", direction="below",
        fire=["Churn fell 40%.", "Revenue grew 12%.", "73% prefer dark mode."],
        not_=["Churn fell 40% according to Stripe.", "Revenue grew 12% per the annual report.",
              "73% prefer dark mode (source: survey)."]))
    cal = validate(s, engine)
    assert cal.passed
    check = Check(spec=s, origin="claude", status="probation", calibration=cal)
    assert check.fires_on(0.05) and not check.fires_on(0.9)


# -- schema --------------------------------------------------------------------
def test_question_shapes():
    with pytest.raises(ValidationError):
        Question(type="noul", instructions="Is this hostile at all?", criteria=["a", "b"])
    with pytest.raises(ValidationError):
        Question(type="choice", instructions="Which role does it play?", criteria=["a", "b"], fire_on=["c"])
    with pytest.raises(ValidationError):  # fire_on can't be every label
        Question(type="choice", instructions="Which role does it play?", criteria=["a", "b"], fire_on=["a", "b"])
    q = Question(type="score", instructions="How urgent is this request?", criteria=["low", "mid", "high"])
    assert signal_from_answer(q, {"score": 1.0}) == 0.5
    c = Question(type="choice", instructions="Which role does it play?", criteria=["a", "b", "c"], fire_on=["a", "c"])
    assert signal_from_answer(c, {"probabilities": {"a": 0.2, "b": 0.5, "c": 0.3}}) == pytest.approx(0.5)


def test_rejects_bad_ids_and_too_few_examples():
    with pytest.raises(ValidationError):
        CheckSpec.model_validate({**spec(), "id": "Bad-ID"})
    with pytest.raises(ValidationError):
        CheckSpec.model_validate({**spec(), "tests": {"should_fire": ["a"], "should_not": ["b", "c", "d"]}})


# -- registry lifecycle --------------------------------------------------------
def _check(id="x_check", status="probation"):
    return Check(spec=CheckSpec.model_validate(spec(id=id)), origin="claude", status=status,
                 calibration=Calibration(passed=True, reason="ok", threshold=0.5))


def test_promotes_after_agreement_and_persists(tmp_path):
    reg = CheckRegistry(tmp_path / "r.json")
    reg.add(_check())
    notes = [reg.add_verification("x_check", Verification(text="t", signal=0.9, applies=True, reason=""))[1]
             for _ in range(3)]
    assert notes[:2] == [None, None] and "promoted" in notes[2]
    assert CheckRegistry(tmp_path / "r.json").get("x_check").status == "active"


def test_retires_when_claude_disagrees(tmp_path):
    reg = CheckRegistry(tmp_path / "r.json")
    reg.add(_check())
    for applies in (False, False, True):
        _, note = reg.add_verification("x_check", Verification(text="t", signal=0.9, applies=applies, reason=""))
    assert reg.get("x_check").status == "probation"  # 3 verdicts isn't enough evidence to retire
    _, note = reg.add_verification("x_check", Verification(text="t", signal=0.9, applies=False, reason=""))
    assert reg.get("x_check").status == "retired" and "retired" in note


def test_recent_verdicts_let_a_refit_rule_recover(tmp_path):
    reg = CheckRegistry(None)
    reg.add(_check())
    for applies, fired in [(True, False), (False, True), (True, True), (True, True), (True, True), (True, True)]:
        _, note = reg.add_verification("x_check", Verification(text="t", signal=0.9, applies=applies,
                                                               fired=fired, reason=""))
    # Two early mistakes, then four matches: 4 of the last 5 -> promoted rather than retired.
    assert reg.get("x_check").status == "active" and "4 of the last 5" in note


def test_promotion_needs_a_confirmed_hit_and_probation_ends(tmp_path):
    reg = CheckRegistry(None)
    reg.add(_check())
    for _ in range(3):  # correct non-fires only: Laya never shown right on a real hit
        reg.add_verification("x_check", Verification(text="t", signal=0.1, applies=False, fired=False, reason=""))
    assert reg.get("x_check").status == "probation"
    for applies in (True, False, True, False, True):  # mixed record, never decisive
        reg.add_verification("x_check", Verification(text="t", signal=0.9, applies=applies, fired=True, reason=""))
        if reg.get("x_check").status != "probation":
            break
    assert reg.get("x_check").status in ("active", "retired")


def test_live_cap_and_unique_ids(tmp_path):
    reg = CheckRegistry(None, max_live=1)
    reg.add(_check("a_check"))
    with pytest.raises(RegistryFull):
        reg.add(_check("b_check"))
    reg.add(_check("c_check", status="rejected"))  # rejected checks don't count toward the cap
    assert reg.unique_id("a_check") == "a_check_2"


# -- escalation ----------------------------------------------------------------
def _flag(kind, fired, uncertain=False):
    return Flag(check_id=kind, label=kind, kind=kind, status="active", severity="low", signal=0.5, threshold=0.5,
                direction="above", fired=fired, uncertain=uncertain)


def test_reasons():
    assert "novel" in reasons_for([_flag("sentinel", True), _flag("rule", False)])[0]
    assert reasons_for([_flag("sentinel", True), _flag("rule", True)]) == []  # a rule already covers it
    assert "uncertain" in reasons_for([_flag("sentinel", False), _flag("rule", False, uncertain=True)])[0]


def test_guard_blocks_repeats_cooldown_and_budget():
    g = EscalationGuard(GuardConfig(max_calls_per_hour=2, paragraph_cooldown_s=60))
    assert g.blocked("h1", "p1", pending=0, now=0) is None
    g.record_call("h1", "p1", now=0)
    assert g.blocked("h1", "p2", pending=0, now=1000) == "already reviewed by Claude"
    assert g.blocked("h2", "p1", pending=0, now=30) == "paragraph is cooling down"
    assert g.blocked("h2", "p1", pending=0, now=90) is None
    assert g.blocked("h3", "p9", pending=5, now=90) == "approval queue is full"
    g.record_call("h4", None, now=10)
    assert g.budget_left(now=20) == 0
    assert g.budget_left(now=3605) == 1  # the call at t=0 has aged out, the one at t=10 has not
    assert g.budget_left(now=3700) == 2
