# AGENTS.md — Open WebUI

Full-stack monorepo: SvelteKit 2 + Svelte 5 frontend (`src/`, adapter-static → `build/`) + FastAPI backend (`backend/open_webui/`, entry `main:app`).

## Dev (run both)

- Frontend: `npm run dev` — Vite on `:5173`, proxies `/api /ollama /openai /oauth /ws` to `WEBUI_BACKEND_URL` or `http://localhost:8080` (see `vite.config.ts`).
- Backend: `sh backend/dev.sh` (sets `CORS_ALLOW_ORIGIN`, `uvicorn open_webui.main:app --reload --port 8080`) or `open-webui dev` after install.
- Requires: Node `18.13–22.x` (`.npmrc` has `engine-strict=true`), Python `>=3.11,<3.13` (`pyproject.toml`). Always install frontend with `npm install --force` / `npm ci --force` (CI + Dockerfile do this; peer deps otherwise fail).
- `npm run dev|build` always runs `scripts/prepare-pyodide.js` first — needs network to fetch Pyodide/ PyPI wheels into `static/pyodide/`. Offline builds will fail there, not in Vite.

## Verify (match CI)

- Frontend CI (`.github/workflows/frontend.yaml`, Node 22): `npm install --force` → `npm run format` → `npm run i18n:parse` → `git diff --exit-code` must be clean → `npm run build` with `NODE_OPTIONS=--max-old-space-size=8192` (build OOMs without it; `hatch_build.py` sets the same).
- Typecheck: `npm run check` (`svelte-kit sync && svelte-check`). Lint: `npm run lint:frontend` (eslint), backend is **ruff, not pylint/black** despite `package.json` script names: `ruff format --check . --exclude .venv --exclude venv` + `ruff check --select=F --ignore=F401,F403,F405,F541,F811,F841 .` (see `backend.yaml`, `.pre-commit-config.yaml`).
- After adding user-facing strings, run `npm run i18n:parse` — CI fails if locales under `src/lib/i18n/locales/` are stale.

## Tests

- Frontend: `npm run test:frontend` (`vitest --passWithNoTests`; only a few `*.test.ts`, e.g. `src/lib/`). Single file: `npx vitest run <path>`.
- Cypress (`npm run cy:open`) needs running backend + frontend; no local config guarantees — prefer vitest for quick checks.
- `regression.yaml` delegates to external `open-webui/tests` repo and only runs on release PRs (`dev`→`main` titled `0.*`/`1.*`). Do not try to run it locally.

## Backend notes

- Config/env: root `.env` loaded via `find_dotenv` in `backend/open_webui/env.py`; copy from `.env.example`. Defaults that bite: `DATA_DIR` is `backend/data/` from source but `open_webui/data/` when run via pip-installed CLI (`FROM_INIT_PY`); `FRONTEND_BUILD_DIR` is repo `build/` from source but `open_webui/frontend/` via pip. Backend serves the frontend build if present, else API-only (`main.py` logs warning) — rebuild frontend (`npm run build`) to see UI changes through the backend/Docker.
- Pip packaging (`hatch_build.py`) runs `npm install --force` + `npm run build` and embeds `build/` as `open_webui/frontend`; import-time `config.py` copies `favicon.png`/`splash.png`/`loader.js` from build into `static/`.
- DB: default SQLite `DATA_DIR/webui.db` (`DATABASE_URL` override); Alembic migrations in `backend/open_webui/migrations/versions/`. Add a migration for schema changes; never hand-edit the db file. `ollama.db` auto-renames to `webui.db` on boot.
- Entry points: CLI `open-webui serve|dev` defined in `backend/open_webui/__init__.py` (handles `WEBUI_SECRET_KEY` file gen, Windows `loop='none'` uvicorn workaround). Routers live in `backend/open_webui/routers/` (one file per domain: `chats.py`, `knowledge.py`, `auths.py`, …); models in `models/`, retrieval/RAG in `retrieval/`.
- Do not upgrade `aiohttp` past pin or `aiodns` to 4.x — pinned for DNS breakage (see comment in `pyproject.toml`).

## BlaskUI desktop shell (`src-tauri/`, Tauri 2)

- Run with `tauri dev` from repo root (Tauri CLI 2.x global). No `devUrl`, no `beforeDevCommand` — the window shows `src-tauri/splash/index.html` first, then Rust navigates it to the backend URL.
- The Rust launcher (`src-tauri/src/main.rs`) spawns `.venv-blaskui` uvicorn on a free port, polls `/health`, checks Ollama (warn-only), and kills the child on window close. Backend python: always `py -3.12` venv; Node here is 22.23.2 via nvm (a v26 in `Program Files` once shadowed it — user PATH fixed so `C:\nvm4w\nodejs` wins).
- Overrides: `BLASKUI_REPO_ROOT`, `BLASKUI_BACKEND_PYTHON`, `BLASKUI_PORT`, `BLASKUI_DATA_DIR`. Full plan/constraints in `BLASKUI_ROADMAP.md` (license: rebrand ok only for ≤50 users, never delete `LICENSE*`).
