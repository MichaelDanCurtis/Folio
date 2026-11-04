"""End-to-end behaviour of the two-way loop with fake Laya and fake Claude."""

import pytest
from fastapi.testclient import TestClient

from laya_loop.engine import text_key
from laya_loop.escalation import EscalationGuard, GuardConfig
from laya_loop.registry import RegistryFull
from laya_loop.server import create_app

from .conftest import FakeBackend, spec, wait_done

OVERCLAIM = "Our launch is guaranteed to succeed and this data proves it."
OVERCLAIM_2 = "The redesign will definitely double revenue."


def analyze(c, *texts):
    return c.analyze([{"id": f"p{i}", "text": t} for i, t in enumerate(texts)])


def pending(c, kind):
    return [q for q in c._queue if q.kind == kind and q.state == "pending"]


def test_full_loop_novel_issue_to_promoted_rule(make_controller):
    backend = FakeBackend()
    c = make_controller(backend)

    # 1. Sentinel fires, no rule covers it -> queued for approval, nothing sent yet.
    res = analyze(c, OVERCLAIM)
    assert "novel" in res["results"]["p0"]["reasons"][0]
    [item] = pending(c, "review")
    assert backend.calls["review"] == 0

    # 2. User approves -> Claude fixes it and writes a check -> Laya validates it -> probation.
    c.approve(item.id)
    assert wait_done(c, item.id).state == "done"
    [s] = c._suggestions
    assert s.rewrite and s.proposals[0].status == "probation"
    assert c.registry.get("overclaims_certainty").status == "probation"
    assert c.rules_version == 1

    # The paragraph Claude wrote the check from is not sent back for verification.
    analyze(c, OVERCLAIM)
    assert pending(c, "verify") == []

    # 3. The new local check now fires on a similar paragraph; since a rule covers it, it is NOT
    #    re-escalated as novel. Instead a verification of the probation check is queued.
    res = analyze(c, OVERCLAIM_2)
    flags = {f["check_id"]: f for f in res["results"]["p0"]["flags"]}
    assert flags["overclaims_certainty"]["fired"]
    assert res["results"]["p0"]["reasons"] == []
    [v] = pending(c, "verify")

    # 4. Claude agrees on three real hits -> promoted to active.
    analyze(c, OVERCLAIM_2, "Nobody could argue with this plan.", "Success is guaranteed this quarter.")
    for q in pending(c, "verify"):
        c.approve(q.id)
        wait_done(c, q.id)
    assert c.registry.get("overclaims_certainty").status == "active"
    assert backend.calls["verify"] == 3


def test_failing_check_is_reworded_once_then_admitted(make_controller):
    bad = spec(id="vague_check", instructions="UNSEPARABLE vague property of the paragraph?")
    good = spec(id="whatever", instructions="Does the text claim something is certain or guaranteed?")
    backend = FakeBackend(checks=[bad], reword={"give_up": False, "note": "", "check": good})
    c = make_controller(backend)
    analyze(c, OVERCLAIM)
    item = pending(c, "review")[0]
    c.approve(item.id)
    wait_done(c, item.id)
    p = c._suggestions[0].proposals[0]
    assert backend.calls["reword"] == 1
    assert p.reworded and p.status == "probation" and p.check_id == "vague_check"


def test_unfixable_and_malformed_checks_are_rejected_and_not_live(make_controller):
    bad = spec(id="vague_check", instructions="UNSEPARABLE vague property of the paragraph?")
    malformed = {**spec(id="broken_check"), "question": {"type": "noul", "instructions": "x"}}
    c = make_controller(FakeBackend(checks=[bad, malformed]))
    analyze(c, OVERCLAIM)
    item = pending(c, "review")[0]
    c.approve(item.id)
    wait_done(c, item.id)
    statuses = [p.status for p in c._suggestions[0].proposals]
    assert statuses == ["rejected", "rejected"]
    assert c.registry.get("vague_check").status == "rejected"
    assert c.registry.get("broken_check") is None
    assert all(ch.id != "vague_check" for ch in c.registry.live())


def test_loop_guards(make_controller):
    backend = FakeBackend(checks=[])
    c = make_controller(backend)

    # Re-analysing the same text never queues it twice.
    analyze(c, OVERCLAIM)
    analyze(c, OVERCLAIM)
    assert len(pending(c, "review")) == 1

    # Dismissed text is not asked about again.
    c.dismiss(pending(c, "review")[0].id)
    analyze(c, OVERCLAIM)
    assert pending(c, "review") == []

    # Claude's own rewrite, even if the sentinel dislikes it, does not bounce back.
    backend2 = FakeBackend(checks=[])
    backend2.review = lambda *a: {"issue_found": True, "issue_summary": "x",
                                  "rewrite": "Still stupid but reworded.", "new_checks": []}
    c2 = make_controller(backend2)
    analyze(c2, "You are stupid.")
    item = pending(c2, "review")[0]
    c2.approve(item.id)
    wait_done(c2, item.id)
    analyze(c2, "Still stupid but reworded.")
    assert pending(c2, "review") == []


def test_pending_items_drop_when_paragraph_changes(make_controller):
    c = make_controller(FakeBackend(checks=[]))
    analyze(c, OVERCLAIM)
    assert len(pending(c, "review")) == 1
    analyze(c, "Our launch is guaranteed to succeed.")  # user kept editing
    [item] = pending(c, "review")
    assert item.text == "Our launch is guaranteed to succeed."
    assert c._queue[0].state == "dismissed"


def test_claude_disagreement_tightens_threshold(make_controller, engine):
    c = make_controller(FakeBackend(applies=False))
    wait_done(c, c.ask("p0", OVERCLAIM).id)
    before = c.registry.get("overclaims_certainty").calibration.threshold  # ~0.475: between 0.05 and 0.9
    safe = "We are likely to win the deal."
    engine.overrides[safe] = 0.6  # Laya over-scores a hedged claim: a false positive at the old threshold
    flags = analyze(c, safe)["results"]["p0"]["flags"]
    assert next(f for f in flags if f["check_id"] == "overclaims_certainty")["fired"]
    v = pending(c, "verify")[0]
    c.approve(v.id)
    wait_done(c, v.id)
    check = c.registry.get("overclaims_certainty")
    assert check.verifications[0].applies is False
    # Claude's verdict became a negative example; the refit threshold now sits above the false positive.
    assert before < 0.6 < check.calibration.threshold
    assert not check.fires_on(0.6) and check.fires_on(0.9)


def test_no_backend_fails_cleanly(make_controller):
    c = make_controller(None)
    analyze(c, OVERCLAIM)
    item = c.approve(pending(c, "review")[0].id)
    assert item.state == "failed" and "no Claude backend" in item.error


def test_claude_sees_existing_checks_to_avoid_duplicates(make_controller):
    backend = FakeBackend()
    c = make_controller(backend)
    item = c.ask("p0", OVERCLAIM)
    wait_done(c, item.id)
    item = c.ask("p1", "You are stupid and ignored me.")
    wait_done(c, item.id)
    ids = {e["id"] for e in backend.last_existing}
    assert "overclaims_certainty" in ids
    # A second proposal with the same id gets a unique suffix rather than overwriting.
    assert c.registry.get("overclaims_certainty_2") is not None


def test_http_api(make_controller):
    c = make_controller(FakeBackend())
    client = TestClient(create_app(c, allowed_hosts=("testserver",)), headers={"X-Laya-Loop": "1"})
    r = client.post("/api/analyze", json={"paragraphs": [{"id": "a", "text": OVERCLAIM}]})
    assert r.status_code == 200 and r.json()["results"]["a"]["reasons"]
    state = client.get("/api/state").json()
    assert state["backend"] == "Fake Claude" and state["queue"][0]["kind"] == "review"
    qid = state["queue"][0]["id"]
    assert client.post(f"/api/queue/{qid}/approve").json()["state"] == "running"
    wait_done(c, qid)
    sid = client.get("/api/state").json()["suggestions"][0]["id"]
    assert client.post(f"/api/suggestions/{sid}/accept").json()["state"] == "accepted"
    assert client.post("/api/checks/overclaims_certainty/enabled", json={"enabled": False}).json()["ok"]
    assert all(ch.id != "overclaims_certainty" for ch in c.registry.live())
    assert client.post("/api/queue/nope/approve").status_code == 404


def test_claude_verdict_on_borderline_rule_teaches_it(make_controller, engine):
    backend = FakeBackend()  # its review says every borderline rule applies
    c = make_controller(backend)
    wait_done(c, c.ask("p0", OVERCLAIM).id)
    check = c.registry.get("overclaims_certainty")
    assert check.calibration.band == 0.15  # threshold ~0.475, so 0.40 is borderline but does not fire
    missed = "The new plan cannot fail."
    engine.overrides[missed] = 0.40

    res = c.analyze([{"id": "p9", "text": missed}])  # p0 is cooling down after the ask above
    assert any("uncertain" in r for r in res["results"]["p9"]["reasons"])
    item = pending(c, "review")[0]
    assert item.borderline == ["overclaims_certainty"]
    c.approve(item.id)
    wait_done(c, item.id)

    check = c.registry.get("overclaims_certainty")
    [v] = check.verifications
    assert v.applies and not v.fired  # Laya missed it; Claude says the rule applies
    assert check.agreement() == (0, 1)
    assert check.fires_on(0.40)  # refit: the missed paragraph is now a positive example
    # Now that the rule fires on that paragraph, it is not sent back for a verdict Claude already gave.
    res = c.analyze([{"id": "p9", "text": missed}])
    flag = next(f for f in res["results"]["p9"]["flags"] if f["check_id"] == "overclaims_certainty")
    assert flag["fired"]
    assert not any(q.check_id == "overclaims_certainty" for q in pending(c, "verify"))


def test_cross_site_posts_and_foreign_hosts_are_refused(make_controller):
    c = make_controller(FakeBackend())
    c.analyze([{"id": "a", "text": OVERCLAIM}])
    qid = pending(c, "review")[0].id
    forged = TestClient(create_app(c, allowed_hosts=("testserver",)))  # no custom header, like a <form> POST
    assert forged.post(f"/api/queue/{qid}/approve").status_code == 403
    assert c._item(qid).state == "pending"  # nothing was sent to Claude
    rebound = TestClient(create_app(c), headers={"X-Laya-Loop": "1"})  # Host: testserver isn't allowed
    assert rebound.get("/api/state").status_code == 400


def test_concurrent_reviews_proposing_the_same_check_both_succeed(make_controller, engine):
    engine.delay = 0.02  # both reviews validate at the same time
    c = make_controller(FakeBackend())
    a = c.ask("pa", OVERCLAIM)
    b = c.ask("pb", "Victory is guaranteed and this proves it.")
    assert wait_done(c, a.id).state == "done" and wait_done(c, b.id).state == "done"
    ids = sorted(p.check_id for s in c._suggestions for p in s.proposals)
    assert ids == ["overclaims_certainty", "overclaims_certainty_2"]


def test_verify_prompt_carries_the_firing_condition(make_controller):
    backend = FakeBackend(checks=[spec(id="names_source", instructions="Does the text name a source for its numbers?",
                                       direction="below",
                                       fire=["Churn fell 40%.", "Revenue grew 12%.", "73% prefer dark mode."],
                                       not_=["Churn fell 40% according to Stripe.",
                                             "Revenue grew 12% per the annual report.",
                                             "73% prefer dark mode (source: survey)."])])
    c = make_controller(backend)
    wait_done(c, c.ask("p0", "Sales rose 30%.").id)
    c.analyze([{"id": "p1", "text": "Costs dropped 15% this year."}])
    v = pending(c, "verify")[0]
    c.approve(v.id)
    wait_done(c, v.id)
    assert "NO" in backend.last_rule["fires_when"]


def test_verify_calls_do_not_block_later_reviews_of_that_text(make_controller):
    c = make_controller(FakeBackend())
    wait_done(c, c.ask("p0", OVERCLAIM).id)
    text = "Nobody could doubt that success is guaranteed."
    c.analyze([{"id": "p1", "text": text}])
    v = pending(c, "verify")[0]
    c.approve(v.id)
    wait_done(c, v.id)
    assert c.guard.blocked(text_key(text), "p2", pending=0) is None


def test_reword_counts_against_budget_and_is_skipped_when_exhausted(make_controller):
    bad = spec(id="vague_check", instructions="UNSEPARABLE vague property of the paragraph?")
    backend = FakeBackend(checks=[bad])
    c = make_controller(backend, guard=EscalationGuard(GuardConfig(max_calls_per_hour=1)))
    item = c.ask("p0", OVERCLAIM)  # manual asks bypass the budget check but still use the one slot
    wait_done(c, item.id)
    assert backend.calls["reword"] == 0
    assert c._suggestions[0].proposals[0].status == "rejected"


def test_reenabling_respects_the_live_cap(make_controller, tmp_path):
    c = make_controller(FakeBackend())
    wait_done(c, c.ask("p0", OVERCLAIM).id)
    c.set_enabled("overclaims_certainty", False)
    c.registry.max_live = len(c.registry.live())
    with pytest.raises(RegistryFull):
        c.set_enabled("overclaims_certainty", True)
