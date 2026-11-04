"""Laya inference with a (text, question) cache, so only edited paragraphs cost anything."""

from __future__ import annotations

import hashlib
import json
import threading
from collections import OrderedDict
from typing import Protocol

from .models import Question


def signal_from_answer(question: Question, answer: dict) -> float:
    """Reduce any Laya answer to one number in [0, 1]."""
    if question.type == "noul":
        return float(answer["noul"])
    if question.type == "score":
        levels = len(question.criteria or [])
        return float(answer["score"]) / (levels - 1)
    probs = answer["probabilities"]
    return float(sum(probs.get(label, 0.0) for label in question.fire_on or []))


def text_key(text: str) -> str:
    # Whitespace-insensitive: the browser's innerText and Claude's raw rewrite can differ in spacing.
    return hashlib.sha256(" ".join(text.split()).encode()).hexdigest()[:16]


def question_key(question: Question) -> str:
    raw = json.dumps(question.model_dump(), sort_keys=True)
    return hashlib.sha256(raw.encode()).hexdigest()[:16]


class Engine(Protocol):
    name: str

    def signals(self, text: str, questions: dict[str, Question]) -> dict[str, float]: ...


class LayaEngine:
    def __init__(self, model_id: str = "aac6fef/laya-mlx", batch_size: int = 32, cache_size: int = 50_000):
        import laya_mlx

        self.name = model_id
        self._agent = laya_mlx.load(model_id, batch_size=batch_size)
        self._lock = threading.Lock()
        self._cache: OrderedDict[tuple[str, str], float] = OrderedDict()
        self._cache_size = cache_size
        self.fresh = 0  # questions actually run through Laya (cache misses)

    def signals(self, text: str, questions: dict[str, Question]) -> dict[str, float]:
        tk = text_key(text)
        out: dict[str, float] = {}
        missing: dict[str, Question] = {}
        with self._lock:
            for name, q in questions.items():
                key = (tk, question_key(q))
                if key in self._cache:
                    self._cache.move_to_end(key)
                    out[name] = self._cache[key]
                else:
                    missing[name] = q
            if missing:
                result = self._agent.predict(text, {n: q.to_laya() for n, q in missing.items()})
                self.fresh += len(missing)
                for name, q in missing.items():
                    value = signal_from_answer(q, result["answers"][name])
                    out[name] = value
                    self._cache[(tk, question_key(q))] = value
                while len(self._cache) > self._cache_size:
                    self._cache.popitem(last=False)
        return out
