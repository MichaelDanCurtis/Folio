"""Local web app: an editor that runs Laya as you type and escalates to Claude with your approval."""

from __future__ import annotations

import argparse
import logging
import os
from pathlib import Path

from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from fastapi.responses import FileResponse, JSONResponse
from pydantic import BaseModel

from .controller import LoopController
from .registry import RegistryFull

WRITE_HEADER = "X-Laya-Loop"  # a custom header forces a CORS preflight, which other origins can't pass

STATIC = Path(__file__).parent / "static"


class Paragraph(BaseModel):
    id: str
    text: str


class AnalyzeRequest(BaseModel):
    paragraphs: list[Paragraph]


class AskRequest(BaseModel):
    paragraph_id: str | None = None
    text: str


class Toggle(BaseModel):
    enabled: bool


def create_app(controller: LoopController, allowed_hosts: tuple[str, ...] = ("127.0.0.1", "localhost")) -> FastAPI:
    app = FastAPI(title="Laya Loop")
    # Rejects DNS-rebinding requests that would read the document through /api/state.
    app.add_middleware(TrustedHostMiddleware, allowed_hosts=list(allowed_hosts))

    @app.middleware("http")
    async def require_write_header(request: Request, call_next):
        # Without this, any web page could POST /api/queue/q1/approve and send text to Claude without a click.
        if request.method == "POST" and request.headers.get(WRITE_HEADER) != "1":
            return JSONResponse({"detail": f"missing {WRITE_HEADER} header"}, status_code=403)
        return await call_next(request)

    @app.get("/")
    def index():
        return FileResponse(STATIC / "index.html")

    @app.post("/api/analyze")
    def analyze(req: AnalyzeRequest):
        return controller.analyze([p.model_dump() for p in req.paragraphs])

    @app.get("/api/state")
    def state():
        return controller.state()

    @app.post("/api/ask")
    def ask(req: AskRequest):
        if len(req.text.strip()) < 3:
            raise HTTPException(400, "paragraph is empty")
        return {"id": controller.ask(req.paragraph_id, req.text).id}

    @app.post("/api/queue/{item_id}/{action}")
    def queue_action(item_id: str, action: str):
        try:
            item = {"approve": controller.approve, "dismiss": controller.dismiss}[action](item_id)
        except KeyError:
            raise HTTPException(404, "unknown item or action")
        return {"state": item.state, "error": item.error}

    @app.post("/api/suggestions/{sid}/{action}")
    def suggestion_action(sid: str, action: str):
        if action not in ("accept", "dismiss"):
            raise HTTPException(404, "unknown action")
        try:
            s = controller.resolve_suggestion(sid, accepted=action == "accept")
        except StopIteration:
            raise HTTPException(404, "unknown suggestion")
        return {"state": s.state}

    @app.post("/api/checks/{check_id}/enabled")
    def set_enabled(check_id: str, body: Toggle):
        if not controller.registry.get(check_id):
            raise HTTPException(404, "unknown check")
        try:
            controller.set_enabled(check_id, body.enabled)
        except RegistryFull as e:
            raise HTTPException(409, str(e))
        return {"ok": True}

    @app.post("/api/checks/{check_id}/retire")
    def retire(check_id: str):
        if not controller.registry.get(check_id):
            raise HTTPException(404, "unknown check")
        controller.retire(check_id)
        return {"ok": True}

    @app.post("/api/settings/auto_verify")
    def auto_verify(body: Toggle):
        controller.auto_verify = body.enabled
        return {"ok": True}

    return app


def main() -> None:
    import uvicorn

    from .claude import make_backend
    from .engine import LayaEngine
    from .registry import CheckRegistry
    from .seeds import RULES, SENTINEL

    parser = argparse.ArgumentParser(description="Laya <-> Claude two-way loop prototype")
    parser.add_argument("--backend", default=os.environ.get("LAYA_LOOP_BACKEND", "auto"),
                        choices=["auto", "api", "cli", "simulated", "none"])
    parser.add_argument("--model", default=os.environ.get("LAYA_LOOP_LAYA_MODEL", "aac6fef/laya-mlx"))
    parser.add_argument("--rules", type=Path, default=Path("rules.json"), help="where learned checks are saved")
    parser.add_argument("--port", type=int, default=8765)
    args = parser.parse_args()

    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
    print(f"Loading Laya ({args.model})...")
    engine = LayaEngine(args.model)
    backend = make_backend(args.backend)
    print(f"Claude backend: {backend.label if backend else 'none (escalation disabled)'}")
    controller = LoopController(engine, CheckRegistry(args.rules), backend)
    controller.bootstrap(RULES, SENTINEL)
    print(f"Open http://127.0.0.1:{args.port}")
    uvicorn.run(create_app(controller), host="127.0.0.1", port=args.port, log_level="warning")


if __name__ == "__main__":
    main()
