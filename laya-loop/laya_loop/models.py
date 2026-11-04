"""Data model for checks: what Claude proposes, what Laya runs, what the UI shows."""

from __future__ import annotations

import re
import time
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, field_validator, model_validator

CHECK_ID_RE = re.compile(r"^[a-z][a-z0-9_]{2,40}$")

Direction = Literal["above", "below"]
Severity = Literal["low", "medium", "high"]
Status = Literal["probation", "active", "retired", "rejected"]
Origin = Literal["seed", "claude"]
Kind = Literal["rule", "sentinel"]


class Question(BaseModel):
    """One Laya typed question. Reduced to a single scalar signal in [0, 1]."""

    model_config = ConfigDict(extra="forbid")

    type: Literal["noul", "score", "choice"]
    instructions: str = Field(min_length=8, max_length=300)
    # score: ordered rubric levels, low to high. choice: option labels.
    criteria: list[str] | None = None
    # choice only: labels whose summed probability is the signal.
    fire_on: list[str] | None = None

    @model_validator(mode="after")
    def _shape(self) -> Question:
        crit = self.criteria or []
        fire = self.fire_on or []
        if any(not c.strip() or len(c) > 60 for c in crit):
            raise ValueError("criteria must be non-empty and at most 60 characters")
        if len(set(crit)) != len(crit):
            raise ValueError("criteria must be unique")
        if self.type == "noul":
            if crit or fire:
                raise ValueError("noul questions take no criteria or fire_on")
            self.criteria = None
            self.fire_on = None
        elif self.type == "score":
            if not 2 <= len(crit) <= 6:
                raise ValueError("score questions need 2-6 ordered criteria")
            if fire:
                raise ValueError("score questions take no fire_on")
            self.fire_on = None
        else:
            if not 2 <= len(crit) <= 8:
                raise ValueError("choice questions need 2-8 criteria")
            if not fire or not set(fire) <= set(crit) or set(fire) == set(crit):
                raise ValueError("choice fire_on must be a non-empty proper subset of criteria")
        return self

    def to_laya(self) -> dict:
        q: dict = {"type": self.type, "instructions": self.instructions}
        if self.criteria:
            q["criteria"] = list(self.criteria)
        return q


class Tests(BaseModel):
    """Examples Claude writes alongside a check; Laya must separate them."""

    model_config = ConfigDict(extra="forbid")

    should_fire: list[str]
    should_not: list[str]

    @field_validator("should_fire", "should_not")
    @classmethod
    def _examples(cls, v: list[str]) -> list[str]:
        v = [s.strip() for s in v if s and s.strip()]
        if not 3 <= len(v) <= 10:
            raise ValueError("need 3-10 examples per side")
        if any(len(s) > 600 for s in v):
            raise ValueError("examples must be at most 600 characters")
        return v


class CheckSpec(BaseModel):
    """A check as authored (by a seed file or by Claude)."""

    model_config = ConfigDict(extra="forbid")

    id: str
    label: str = Field(min_length=3, max_length=120)
    rationale: str = Field(default="", max_length=400)
    severity: Severity = "medium"
    direction: Direction = "above"
    question: Question
    tests: Tests

    @field_validator("id")
    @classmethod
    def _id(cls, v: str) -> str:
        if not CHECK_ID_RE.match(v):
            raise ValueError("id must be snake_case, 3-41 chars, starting with a letter")
        return v


class Calibration(BaseModel):
    """Result of running a check's examples through Laya.

    `threshold` and `band` live in *oriented* signal space: the check fires when
    oriented(signal) >= threshold, where oriented flips the signal for
    direction="below" so every check reads "higher means fire".
    """

    passed: bool
    reason: str
    threshold: float = 0.5
    band: float = 0.05
    accuracy: float = 0.0
    margin: float = 0.0
    pos_signals: list[float] = []
    neg_signals: list[float] = []
    examples_used: int = 0


class Verification(BaseModel):
    """Claude's verdict on real text. Laya was right when applies == fired."""

    text: str
    signal: float
    applies: bool
    reason: str
    fired: bool = True
    at: float = Field(default_factory=time.time)


class Check(BaseModel):
    spec: CheckSpec
    kind: Kind = "rule"
    origin: Origin
    status: Status
    enabled: bool = True
    calibration: Calibration
    verifications: list[Verification] = []
    fires: int = 0
    created_at: float = Field(default_factory=time.time)
    history: list[str] = []

    @property
    def id(self) -> str:
        return self.spec.id

    @property
    def live(self) -> bool:
        return self.enabled and self.status in ("probation", "active")

    def oriented(self, signal: float) -> float:
        return signal if self.spec.direction == "above" else 1.0 - signal

    def fires_on(self, signal: float) -> bool:
        return self.oriented(signal) >= self.calibration.threshold

    def uncertain_on(self, signal: float) -> bool:
        return abs(self.oriented(signal) - self.calibration.threshold) < self.calibration.band

    def raw_threshold(self) -> float:
        t = self.calibration.threshold
        return t if self.spec.direction == "above" else 1.0 - t

    def fires_when(self) -> str:
        """The firing condition in words, so Claude judges the rule rather than Laya's raw question."""
        q, above = self.spec.question, self.spec.direction == "above"
        if q.type == "noul":
            return f"the answer to Laya's question is {'YES' if above else 'NO'}"
        if q.type == "score":
            return f"the rating is {'high' if above else 'low'} on the scale {q.criteria} (low to high)"
        return f"the answer {'is' if above else 'is not'} one of {q.fire_on}"

    def describe(self) -> dict:
        return {"id": self.id, "label": self.spec.label, "question": self.spec.question.instructions,
                "fires_when": self.fires_when()}

    def agreement(self, last: int | None = None) -> tuple[int, int]:
        """How often Laya (with the threshold it had at the time) matched Claude's verdict."""
        recent = self.verifications[-last:] if last else self.verifications
        return sum(1 for v in recent if v.applies == v.fired), len(recent)


class Flag(BaseModel):
    check_id: str
    label: str
    kind: Kind
    status: Status
    severity: Severity
    signal: float
    threshold: float  # raw space, for display
    direction: Direction
    fired: bool
    uncertain: bool
