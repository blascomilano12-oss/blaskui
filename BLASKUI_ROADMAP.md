# BlaskUI — Roadmap ufficiale (leggere sempre prima di lavorare)

> Progetto: trasformare Open WebUI in **BlaskUI**, app desktop Windows con Tauri.
> Regola: il backend resta intatto, si personalizza solo il frontend + shell desktop.
> Lingua di lavoro con l'utente: sempre italiano.

## 0. Decisione licenza (VERIFICATO, non toccare)

- `LICENSE` punto 4 vieta di rimuovere il branding "Open WebUI" **tranne** se gli utenti finali sono max 50 in 30 giorni, oppure con permesso scritto / licenza enterprise.
- Uso di BlaskUI = solo personale, nessuna distribuzione → **rientra nell'eccezione (i)**: rebrand consentito.
- **NON eliminare mai** i file `LICENSE`, `LICENSE_HISTORY`, `CONTRIBUTOR_LICENSE_AGREEMENT` né i copyright notice nel codice. Tenerli è obbligatorio anche per uso personale.
- Se mai distribuirai a più di 50 persone → fermarsi e chiedere permesso scritto o licenza enterprise.

## 1. Architettura target BlaskUI

```
BlaskUI (Tauri, Windows)
├── Shell Tauri (`src-tauri/`) — apre finestra WebView, lancia/controlla backend
├── Frontend BlaskUI (fork di `src/` → build in `build/`) — nome, tema, funzioni custom
└── Backend originale (`backend/open_webui/`, intatto) — FastAPI su porta locale dinamica
     ├── Ollama locale (`OLLAMA_BASE_URL=http://localhost:11434`, prerequisito esterno)
     └── OpencodeZEN (endpoint OpenAI-compatibile via `OPENAI_API_BASE_URL` + KEY)
```

- Scelta definitiva: **Tauri** (leggero), non Electron, non `open-webui/desktop`.
- Backend **non incorporato** come sidecar PyInstaller nella v1 (troppo pesante: torch/transformers/chroma = GB). Strategia v1: Tauri pretende `open-webui serve` installato oppure lo installa, lo avvia e aspetta `/health`.
- Frontend compilato con adapter-static (`npm run build` → `build/`, fallback `index.html`) e caricato dalla shell.

## 2. Fasi (in ordine, non saltare)

### Fase 0 — Ambiente (toolchain VERIFICATA il 2026-09-17)

- Node **22.23.2** = default di sistema via nvm (`C:\nvm4w\nodejs`, PATH utente sistemato perché `C:\Program Files\nodejs` con v26 lo oscurava in alcune shell). NON usare Node 24/26 qui. Comando: `nvm use 22.23.2`.
- Python backend: usare sempre **`py -3.12`** (3.12.10 in `AppData\Local\Programs\Python\Python312`). Il comando `python` punta al 3.14 e NON va usato per il backend (richiesto >=3.11,<3.13).
- Rust/cargo 1.97 ✅ · Tauri CLI **2.11.4** globale (`npm install -g @tauri-apps/cli`, binario `tauri`) ✅
- [x] `npm install --force` eseguito e `npm run build` ok (build/ = 0.24 GB) — 2026-09-17
- [x] Backend: venv `.venv-blaskui` con `py -3.12` + `backend/requirements.txt` completo installato (**2.50 GB reali**), smoke test import ok (`VERSION 0.11.3`, `DATA_DIR=backend/data`, `FRONTEND=build/`). Nota: l'import pretende `WEBUI_SECRET_KEY` (metterne una finta per i test, gli script di avvio la generano da soli).

### Fase 1 — Shell Tauri minima (VERIFICATA il 2026-09-17, funzionante)

- [x] `src-tauri/` creato: `tauri.conf.json` (app `BlaskUI`, finestra 1280x800 min 940x600), `src/main.rs` (launcher), `splash/index.html` (splash con stato), `capabilities/default.json` (vuota, nessuna API JS usata), icone generate con `tauri icon build/static/favicon.png`
- [x] All'avvio: porta libera → lancia `.venv-blaskui` uvicorn → poll `/health` (timeout 300s, messaggi di stato) → naviga finestra su backend. Chiave `WEBUI_SECRET_KEY` letta/generata in `backend/.webui_secret_key` (come start.sh).
- [x] Ollama assente: messaggio in splash + si prosegue (verificato: Ollama NON installato su questo PC).
- [x] Chiusura finestra = kill backend figlio (`BackendState`, `WindowEvent::Destroyed`). Override utili: `BLASKUI_REPO_ROOT`, `BLASKUI_BACKEND_PYTHON`, `BLASKUI_PORT`, `BLASKUI_DATA_DIR`.
- [x] `tauri dev` verificato: app compila, backend boot ~20s, finestra carica tutta la UI (`GET / 200`, `/api/config 200`). Nota: `tauri dev` resta in esecuzione (è normale); chiudere la finestra killa il backend, poi Ctrl+C nel terminale.
- Prossimo: ~~installare Ollama (Fase 3) per la chat con modelli locali.~~ Ollama VERIFICATO il 2026-09-17: installato in `AppData\Local\Programs\Ollama`, serve avviato (pid dedicato, log in `%TEMP%\ollama-serve.*`), 11 modelli presenti tra cui 4 custom `blask-*` + qwen/devstral/gemma. Nota: deve restare in esecuzione (`ollama serve`); la shell BlaskUI lo rileva su :11434, se spento mostra solo l'avviso e prosegue.

### Fase 2 — Fork visivo BlaskUI (PWA brandizzata 2026-09-20, tema ancora aperto)

- [x] Branding PWA: `WEBUI_NAME=BlaskUI` (`env.py:936`), `static/static/site.webmanifest:2-3` `name:BlaskUI`, `static/opensearch.xml:3-4` `ShortName:BlaskUI`, `backend/env.py:944` `WEBUI_FAVICON_URL` locale, `build/static/site.webmanifest` + `build/opensearch.xml` patchati e payload aggiornato (973,8 MB) — verificato in runtime `21:42`
- [ ] Tema: `tailwind.config.js`, `src/app.css` / `src/tailwind.css`, componenti `src/lib/components/`
- [ ] Verifiche: `npm run format` → `npm run i18n:parse` → `git diff --exit-code` pulito → `npm run check` → `npm run build`
- Criterio uscita: la shell Fase 1 carica il `build/` brandizzato BlaskUI, nessuna regressione funzionale — *parziale: PWA ok, tema ancora da fare*

### Fase 3 — Config BlaskUI (VERIFICATA il 2026-09-17, resta 1 azione utente)

- [x] `.env` attivo: `OLLAMA_BASE_URL=localhost:11434`, `OPENAI_API_BASE_URL=https://opencode.ai/zen/v1` + key, `DATA_DIR=backend/data-blaskui` dedicato, `WEBUI_NAME=BlaskUI`. Chiave valida (`/v1/models` → 11 modelli free).
- [x] Backend con `.env` nuovo: boot ok + migrazioni su DB fresco, signup admin `admin@blaskui.local` creato (password solo locale: `blaskui-admin-01`, da cambiare).
- [x] Filo Ollama end-to-end: `/ollama/api/tags` via backend → 11 modelli locali ✅
- [x] Filo Zen end-to-end: `/openai/models` via backend → 11 modelli ✅
- [ ] AZIONE UTENTE: chat Zen bloccata da `401 CreditsError "No payment method"` — aggiungere metodo di pagamento su opencode.ai billing, poi la chat si sblocca da sola (nessuna modifica codice). In Open WebUI funzionano solo i modelli su `/v1/chat/completions` (deepseek/glm/mimo/ling/nemotron free); GPT/Grok/Muse (`/responses`) e Claude (`/messages`) usano altre API e non passano dalla connessione OpenAI.
- Gotcha scoperto: `DATA_DIR` deve esistere già (sqlite non crea le cartelle) — creata `backend/data-blaskui`; il launcher Fase 4 dovrà crearla da solo.

## Piano Zen → OpenRouter (ESEGUITO il 18/09, resta 1 azione utente)

- `.env` su OpenRouter + backup `.env.bak-zen`. Scoperta importante: le connessioni OpenAI vivono nel DB (`openai.api_base_urls`), NON rileggono `.env` ai restart — aggiornato via `POST /openai/config/update` (446 modelli, free inclusi).
- Chat bloccata solo dai guardrail dell'account OpenRouter (404 su endpoint free): UTENTE HA SISTEMATO (training + provider allowlist) — verificato 18/09: stream live ok da OpenRouter diretto. Se ricapita, ricontrollare i guardrail per primo.
- Utenti di test creati per le verifiche ed ELIMINATI (DB pulito, solo admin).
- `scripts/blaskui-dev.ps1` ora ha `-Visible` (finestre PowerShell visibili, niente prompt nascosti).

- Causa provata (screenshot): i modelli Zen free rispondono `OpenCode's free tier can only be used from within OpenCode` — da fuori OpenCode sono inutilizzabili. Spiega tutto: niente token live (solo evento errore) + testo d'errore salvato come messaggio e visibile al revisit. Stessa classe dei precedenti `Model is unavailable` / `401 No payment method`.
- Path Ollama (locale e cloud) verificati perfetti: 1550/1550 righe protocollo ok, 1762 eventi SSE backend con 0 anomalie — il problema NON è BlaskUI.
- Direzione (proposta utente): passare a **OpenRouter** (`https://openrouter.ai/api/v1`, OpenAI-compatibile, modelli `:free` usabili dall'esterno). Solo `.env` + restart, zero modifiche codice. Dettaglio passi nel messaggio di piano; eseguire solo dopo ok utente + chiave nuova.

### Fase 4 — Installer Windows (VERIFICATO 2026-09-20, hotfix atomico)

- [x] Tauri bundler `tauri build` → `BlaskUI_setup.exe` (NSIS 950,6 MB, 20/09 21:35) — payload 973,8 MB con `CHANGELOG.md` + `build` + `.venv-blaskui`, `tauri build` patch resources `../payload/payload.zip`
- [x] Test su cartella pulita **PASSATO** 2026-09-20 21:42-21:45: wipe `%LOCALAPPDATA%\BlaskUI\runtime+data` → install `/S` → launch `blaskui.exe` → estrazione atomica 53s (52094 file, 2078 MB) → commit `backend+.venv+build` + marker `.blaskui-payload-v1` → backend `uvicorn` su 49612 → `/health 200`, `/api/config WEBUI_NAME=BlaskUI`, `/api/v1/auths/signup admin@blaskui.local OK`, `/ollama/api/tags 11 modelli` (gemma4, minimax-m3, gpt-oss…), `build/static/site.webmanifest` BlaskUI, `backend/env.py` favicon locale. Hotfix `src-tauri/src/main.rs:111-185` verificato.
- Criterio uscita: installer funzionante su Windows senza repo/Node visibili all'utente — **RAGGIUNTO**

### Fase 5 — BlaskUI v0.2: wizard, brand, motion, Studio (IN CORSO notte 28→29/09)

- [x] Wizard `/welcome` (5 passi, solo `onboarding:true`, animato motion/mini) — verificato in browser end-to-end
- [x] Pagina `/accounts` stile Netflix + password stile MacBook + remember hook (auth/welcome) — verificata in browser
- [x] Seed build: Blasco/Blask01 (admin, solo DB di test; a nuovo installer ognuno crea il suo)
- [x] Cursore custom globale (molla + tilt + tag nome utente, solo mouse) — verificato in browser
- [x] Input chat: morph CSS (`blask-pop` + lift al focus), VoiceRecording invariata (ha già le barre), picker intatto
- [x] Picker: tab Tutti/BlaskModel/Locale/Cloud/Free (BlaskModel = connessione taggata `blask`)
- [x] Font Satoshi bundled (300/400/500/700) + fallback Inter; video `blask-bg.mp4` 8 MB in splash/wizard/auth
- [x] Logo BlaskUI: `static/assets/brand/` (SVG + animato + PNG 16-512 + ICO), sidebar/auth/wizard/splash/favicon
- [x] Launcher: avvio/adozione Unsloth Studio + idle-off 20 min + `unsloth.json`; splash con video+logo
- [x] Update-check Open WebUI spento (`ENABLE_VERSION_UPDATE_CHECK=false`); STT whisper env pronte (verifica rimandata: valore vecchio cachato nel DB di test, fix codice `engine in ('','whisper')` dentro)
- [x] i18n: 1372/3132 tradotte (MT gemma4 morta di timeout, resto in fallback EN); `DEFAULT_LOCALE=it-IT`
- [ ] Payload + installer v0.2 rebuild (in corso) — poi test installazione pulita
- Nota Spark-X2.5: Ollama lo rifiuta (`llama-quantize` exit 1) → va servito da Unsloth Studio `/v1`, MiniCPM-F16 ok in Ollama (`blask-minicpm-test`)

### Fase 6 — Funzioni custom (solo dopo Fase 5 verde)

- [ ] STUDIO idea utente (team blask-\* + sezione IDE tipo Antigravity/VS Code) — analisi senior:
  - Base già nel repo: chat multi-modello in parallelo, Tools/Functions/Pipes, Channels realtime con tag modelli, artifact KV storage, `@xterm` + `@xyflow/svelte` già nelle dipendenze frontend, RAG/knowledge per contesto progetto.
  - Opzione A (consigliata, incrementale): orchestratore come Workspace Function/Pipeline che smista task ai 4 modelli via API Ollama interna + pagina custom in `src/routes/` stile IDE (file tree + editor + terminale xterm + pannello team con output per-agente in streaming). Zero modifiche al core backend.
  - Opzione B (più spinta): Channel dedicato con i 4 agenti invitati che dialogano in realtime + artifact condiviso come "file di progetto".
  - Nodi da sciogliere: 4 modelli da 7-15GB in parallelo = RAM/VRAM (serve coda/turni), formato di scambio (artifact JSON vs chat), sandbox esecuzione codice (Pyodide già integrato nel frontend vs terminale esterno).
  - Decisione dopo Fase 4; prototipo = A minimale (1 task → 4 modelli → merge risposte).
- [ ] Una feature alla volta, prima solo frontend (`src/routes/`, `src/lib/`); nuovo backend solo se serve (nuovo file in `backend/open_webui/routers/`, mai editare quelli esistenti se evitabile)
- [ ] Migrazioni DB via Alembic se cambia schema; mai editare `webui.db` a mano
- Criterio uscita per ogni feature: typecheck + build + test manuale in app packaggiata

## 3. Comandi che contano (non indovinare)

- ATTENZIONE RAM: `npm run build` vuole ~6-8 GB liberi (heap 8192). Con poca RAM muore in silenzio senza errore. Prima di buildare: chiudere browser/app pesanti e stoppare backend orfani (`python -m uvicorn open_webui`). Verificare con Task Manager o `Get-CimInstance Win32_OperatingSystem`.

- Frontend dev: `npm run dev` (proxy → :8080) · Backend dev: `sh backend/dev.sh`
- Install frontend: sempre `npm install --force` / `npm ci --force`
- Verifica pre-commit frontend: `npm run format`, `npm run i18n:parse`, `git diff --exit-code`, `npm run check`, `npm run build` (con `NODE_OPTIONS=--max-old-space-size=8192`)
- Backend lint (CI): `ruff format --check . --exclude .venv --exclude venv` + `ruff check --select=F --ignore=F401,F403,F405,F541,F811,F841 .`
- Test rapidi: `npm run test:frontend` · singolo: `npx vitest run <path>`
- Backend serve installato: `open-webui serve --port <X>` · health: `GET /health`

## 4. Vietato

- Cancellare/modificare `LICENSE*`, copyright header, o nascondere l'attribuzione.
- Incorporare Ollama o i modelli dentro l'installer v1.
- Modificare router/modelli backend esistenti per motivi estetici.
- Saltare fasi o aggiungere feature prima che la Fase 4 sia verde.

## Build macOS (in corso)

- Workflow: .github/workflows/build-macos.yml (push su master o dispatch manuale)
- Ultimo push: 2026-09-30 23:48

