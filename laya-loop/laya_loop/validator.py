"""Test-before-activate: run a check's examples through Laya and learn its threshold.

Laya's raw probabilities are not calibrated for arbitrary questions (a hostile
paragraph may score 0.3 while neutral ones score 0.03), so a fixed 0.5 cutoff is
useless. Instead each check gets the threshold that best separates its own
should_fire / should_not examples, and is rejected if no threshold separates
them well enough.
"""

from __future__ import annotations

from statistics import mean

from .engine import Engine
from .models import Calibration, CheckSpec

MIN_ACCURACY = 0.85
MIN_MARGIN = 0.10  # mean(pos) - mean(neg), oriented
MIN_BAND, MAX_BAND = 0.02, 0.15


def oriented(signal: float, direction: str) -> float:
    return signal if direction == "above" else 1.0 - signal


def calibrate(pos: list[float], neg: list[float]) -> Calibration:
    """pos/neg are oriented signals (higher should mean fire)."""
    if len(pos) < 3 or len(neg) < 3:
        return Calibration(passed=False, reason="need at least 3 examples per side",
                           pos_signals=pos, neg_signals=neg)

    values = sorted(set(pos + neg))
    candidates = [(values[i] + values[i + 1]) / 2 for i in range(len(values) - 1)] or [values[0]]
    total = len(pos) + len(neg)

    def score(t: float) -> tuple[float, float]:
        acc = (sum(p >= t for p in pos) + sum(n < t for n in neg)) / total
        below = [v for v in values if v < t]
        above = [v for v in values if v >= t]
        gap = (min(above) - max(below)) if below and above else 0.0
        return acc, gap

    threshold = max(candidates, key=score)
    accuracy, gap = score(threshold)
    margin = mean(pos) - mean(neg)
    band = min(MAX_BAND, max(MIN_BAND, gap / 2))

    if accuracy < MIN_ACCURACY:
        passed, reason = False, f"Laya only separates {accuracy:.0%} of the examples (needs {MIN_ACCURACY:.0%})"
    elif margin < MIN_MARGIN:
        passed, reason = False, f"examples barely differ to Laya (margin {margin:+.2f}, needs {MIN_MARGIN:+.2f})"
    else:
        passed, reason = True, f"separates {accuracy:.0%} of examples, margin {margin:+.2f}"

    return Calibration(passed=passed, reason=reason, threshold=round(threshold, 4), band=round(band, 4),
                       accuracy=round(accuracy, 4), margin=round(margin, 4),
                       pos_signals=[round(p, 4) for p in pos], neg_signals=[round(n, 4) for n in neg],
                       examples_used=total)


def validate(spec: CheckSpec, engine: Engine,
             extra_pos: list[str] = (), extra_neg: list[str] = ()) -> Calibration:
    """Run the spec's tests (plus any verified real paragraphs) through Laya."""
    q = {"q": spec.question}
    pos = [oriented(engine.signals(t, q)["q"], spec.direction) for t in [*spec.tests.should_fire, *extra_pos]]
    neg = [oriented(engine.signals(t, q)["q"], spec.direction) for t in [*spec.tests.should_not, *extra_neg]]
    return calibrate(pos, neg)
