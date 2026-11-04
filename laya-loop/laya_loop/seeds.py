"""Starting checks. They go through the same validator as Claude's, so thresholds are learned, not guessed."""

from .models import CheckSpec

SENTINEL = CheckSpec.model_validate({
    "id": "general_problem",
    "label": "General problem detector (routes new kinds of issue to Claude)",
    "rationale": "Weak on its own; its job is to notice issues no rule covers yet.",
    "severity": "low",
    "question": {"type": "noul", "instructions":
                 "Does this paragraph contain a problem that could embarrass the author or cause harm if sent?"},
    "tests": {
        "should_fire": [
            "Per my last three emails, which you clearly ignored, this is late.",
            "The admin password for the staging server is hunter2, use it for the demo.",
            "Our competitor's product is garbage and their CEO is a fraud.",
            "Honestly the client is too stupid to notice if we skip the testing.",
        ],
        "should_not": [
            "Thanks for the draft; I left a few notes on section two.",
            "The demo environment is ready; credentials are in the team vault.",
            "We plan to ship the export feature in the next release, pending QA.",
            "Competitor X offers a lower entry price but fewer integrations.",
        ],
    },
})

RULES = [
    CheckSpec.model_validate({
        "id": "hostile_tone",
        "label": "Flag hostile, insulting or passive-aggressive tone",
        "severity": "high",
        "question": {"type": "noul", "instructions": "Is the tone hostile, insulting or passive-aggressive?"},
        "tests": {
            "should_fire": [
                "Frankly, whoever wrote the last spec should be embarrassed.",
                "Per my last three emails, which you clearly ignored, this is late.",
                "This is the laziest proposal I have read all year.",
                "I guess reading the brief was optional for your team.",
            ],
            "should_not": [
                "Thanks for the draft, a few small notes below.",
                "The spec needs another pass on section 2.",
                "Could we move the review to Thursday?",
                "I disagree with the pricing section; here is why.",
            ],
        },
    }),
    CheckSpec.model_validate({
        "id": "promises_dated_delivery",
        "label": "Flag promises to deliver something by a specific date",
        "rationale": "Dated commitments should be deliberate; this makes them visible.",
        "severity": "medium",
        "question": {"type": "noul", "instructions": "Does the text promise to deliver something by a specific date?"},
        "tests": {
            "should_fire": [
                "We will ship the export feature by March 3.",
                "I'll have the report to you by Friday.",
                "The team commits to launching on June 1.",
                "Expect the signed contract on your desk by the 15th.",
            ],
            "should_not": [
                "We might look at exports at some point.",
                "The report covers Q3 results.",
                "Launch timing is still being discussed.",
                "The contract was signed last year.",
            ],
        },
    }),
]
