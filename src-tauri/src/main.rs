#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! BlaskUI â€” shell desktop (Fase 1).
//! All'avvio: trova una porta libera, lancia il backend FastAPI dal venv,
//! aspetta GET /health, controlla Ollama (senza bloccare se manca) e poi
//! naviga la finestra principale sull'URL del backend.
//! Alla chiusura della finestra il processo backend viene terminato.

use base64::Engine as _;
use rand::RngCore;
use std::io::BufRead as _;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{Manager, WindowEvent};

struct BackendState(Mutex<Option<Child>>);
struct ComfyUIState(Mutex<Option<Child>>);
struct OllamaState(Mutex<Option<Child>>);
/// Studio locale (Unsloth): processo, porta e ultima attività osservata.
struct StudioState {
    child: Mutex<Option<Child>>,
    port: Mutex<Option<u16>>,
    /// Ultimo momento in cui l'utente ha usato Studio o la finestra BlaskUI.
    last_active: Mutex<Instant>,
}

/// Minuti di inattività dopo i quali Studio viene spento (stile "login MacBook").
const STUDIO_IDLE_MINUTES: u64 = 20;

/// Radice del progetto.
/// 1) Override: BLASKUI_REPO_ROOT.
/// 2) Installazione (Fase 4): la cartella runtime/ estratta dal payload (o la
///    ResourceDir Tauri) che contiene backend/, .venv-blaskui/ e build/.
/// 3) Dev: parent di src-tauri/ (dove vivono backend/, build/, .venv-blaskui/).
fn repo_root(app: &tauri::AppHandle) -> PathBuf {
    if let Ok(p) = std::env::var("BLASKUI_REPO_ROOT") {
        return PathBuf::from(p);
    }

    if let Some(runtime) = runtime_dir() {
        if runtime.join("backend").join("open_webui").join("main.py").exists()
            && runtime.join(".venv-blaskui").join("Scripts").join("python.exe").exists()
        {
            return runtime;
        }
    }

    let mut candidates = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.to_path_buf());
        }
    }
    for c in candidates {
        if c.join("backend").join("open_webui").join("main.py").exists()
            && c.join(".venv-blaskui").join("Scripts").join("python.exe").exists()
        {
            return c;
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}

/// Base directory dell'installazione (dati utente + runtime estratto).
/// In dev Ã¨ vuota (si usa il repo). Default: %LOCALAPPDATA%\BlaskUI.
fn install_base() -> Option<PathBuf> {
    if std::env::var("BLASKUI_REPO_ROOT").is_ok() {
        return None;
    }
    if let Ok(p) = std::env::var("BLASKUI_DATA_DIR") {
        return Some(PathBuf::from(p));
    }
    let local =
        std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or(std::env::temp_dir());
    Some(local.join("BlaskUI"))
}

/// Cartella in cui il launcher estrae il payload (backend + venv + build).
fn runtime_dir() -> Option<PathBuf> {
    install_base().map(|b| b.join("runtime"))
}

/// True quando il root Ã¨ quello di un'installazione (resources Tauri o
/// manifesto estratto), non il layout di sviluppo (parent di src-tauri/).
fn is_installed(app: &tauri::AppHandle) -> bool {
    if std::env::var("BLASKUI_REPO_ROOT").is_ok() {
        return false; // override esplicito: trattalo come dev
    }
    // 1) c'Ã¨ il payload zip pronto da estrarre nella resource dir
    if let Ok(res) = app.path().resource_dir() {
        if res.join("payload.zip").exists() {
            return true;
        }
    }
    // 2) il payload Ã¨ giÃ  stato estratto a runtime, ma SOLO se l'estrazione
    //    atomica ha scritto il marker di completamento (falso positivo = zip
    //    troncato a metÃ  o estrazione interrotta). Senza marker -> ri-estrai.
    if let Some(runtime) = runtime_dir() {
        if runtime.join(".blaskui-payload-v1").exists()
            && runtime.join("backend").join("open_webui").join("main.py").exists()
            && runtime.join(".venv-blaskui").join("Scripts").join("python.exe").exists()
        {
            return true;
        }
    }
    false
}

/// Estrae payload.zip (nella resource dir) dentro runtime_dir().
/// No-op se il backend risulta giÃ  estratto.
fn extract_payload(app: &tauri::AppHandle, report: impl Fn(&str)) -> Result<(), String> {
    if let Some(runtime) = runtime_dir() {
        if runtime.join(".blaskui-payload-v1").exists()
            && runtime.join("backend").join("open_webui").join("main.py").exists()
            && runtime.join(".venv-blaskui").join("Scripts").join("python.exe").exists()
        {
            return Ok(());
        }
    }
    let res = app
        .path()
        .resource_dir()
        .map_err(|e| format!("resource dir non disponibile: {e}"))?;
    let zip_path = res.join("payload.zip");
    if !zip_path.exists() {
        return Err("payload.zip non trovato nelle risorse".to_string());
    }
    let runtime = runtime_dir().ok_or("runtime dir non risolvibile")?;
    std::fs::create_dir_all(&runtime).map_err(|e| format!("crea runtime: {e}"))?;

    // Estrazione ATOMICA: si scrive in una dir temporanea e si rinomina solo a
    // lavoro finito. Evita il falso positivo di payload troncato/interrotto.
    let tmp = runtime.join(".blaskui-extracting");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).map_err(|e| format!("pulisci tmp: {e}"))?;
    }
    std::fs::create_dir_all(&tmp).map_err(|e| format!("crea tmp: {e}"))?;

    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("apri payload.zip: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("legge payload.zip: {e}"))?;

    let total = archive.len();
    for i in 0..total {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("entry {i}: {e}"))?;
        let Some(name) = entry.enclosed_name().map(|n| n.to_path_buf()) else {
            continue; // zip-slip-safe: salta i path fuori dalla cartella
        };
        let out = tmp.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| format!("mkdir {name:?}: {e}"))?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {parent:?}: {e}"))?;
            }
            let mut o = std::fs::File::create(&out)
                .map_err(|e| format!("crea {name:?}: {e}"))?;
            std::io::copy(&mut entry, &mut o)
                .map_err(|e| format!("scrivi {name:?}: {e}"))?;
        }
        if i % 500 == 0 {
            report(&format!("Estrazione payload: {}/{} elementiâ€¦", i, total));
        }
    }

    // COMMIT ATOMICO: sposta tutte le top-dir del payload (backend, .venv-blaskui, build)
    // da tmp a runtime con rename atomico per dir; solo dopo scrivi marker.
    // is_installed() richiede marker -> mai runtime parziale visibile.
    // ponytail: marker statico ok per v1, hash payload quando servirà invalidare update.
    drop(archive);
    let tops = ["backend", ".venv-blaskui", "build"];
    let mut found_backend = false;
    for name in tops {
        let staged = tmp.join(name);
        if !staged.exists() {
            continue;
        }
        if name == "backend" {
            found_backend = true;
        }
        let final_dir = runtime.join(name);
        if final_dir.exists() {
            std::fs::remove_dir_all(&final_dir)
                .map_err(|e| format!("rimuovi {name} vecchio: {e}"))?;
        }
        std::fs::rename(&staged, &final_dir)
            .map_err(|e| format!("commit {name}: {e}"))?;
    }
    if !found_backend {
        return Err("payload senza cartella backend/".to_string());
    }
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::write(runtime.join(".blaskui-payload-v1"), b"ok")
        .map_err(|e| format!("scrivi marker: {e}"))?;
    Ok(())
}

/// Python del venv backend. Override: BLASKUI_BACKEND_PYTHON.
fn backend_python(root: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("BLASKUI_BACKEND_PYTHON") {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        root.join(".venv-blaskui").join("Scripts").join("python.exe")
    } else {
        root.join(".venv-blaskui").join("bin").join("python")
    }
}

/// Porta libera su 127.0.0.1. Override: BLASKUI_PORT.
fn find_free_port() -> u16 {
    if let Ok(p) = std::env::var("BLASKUI_PORT") {
        if let Ok(port) = p.parse::<u16>() {
            return port;
        }
    }
    TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(8080)
}

/// Legge backend/.webui_secret_key oppure la genera (come start.sh) e la salva.
fn ensure_secret_key(backend_dir: &Path) -> String {
    let key_file = backend_dir.join(".webui_secret_key");
    if let Ok(s) = std::fs::read_to_string(&key_file) {
        let s = s.trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    let mut buf = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut buf);
    let key = base64::engine::general_purpose::STANDARD.encode(buf);
    let _ = std::fs::write(&key_file, &key);
    key
}

fn set_status(app: &tauri::AppHandle, msg: &str) {
    eprintln!("[blaskui] {msg}");
    if let Some(w) = app.get_webview_window("main") {
        let js = format!(
            "var el=document.getElementById('status'); if(el) el.textContent = {};",
            serde_json::to_string(msg).unwrap_or_else(|_| "\"...\"".to_string())
        );
        let _ = w.eval(&js);
    }
}

/// Aggiorna la barra di progresso della splash (0-100).
fn set_progress(app: &tauri::AppHandle, pct: u8) {
    if let Some(w) = app.get_webview_window("main") {
        let js = format!(
            "var b=document.getElementById('progress-bar'); var p=document.getElementById('progress-pct'); if(b) b.style.width='{pct}%'; if(p) p.textContent='{pct}%';"
        );
        let _ = w.eval(&js);
    }
}

fn pipe_logs(pipe: Option<impl std::io::Read + Send + 'static>, tag: &'static str) {
    if let Some(p) = pipe {
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(p).lines().map_while(Result::ok) {
                eprintln!("[backend:{tag}] {line}");
            }
        });
    }
}

fn ollama_available(client: &reqwest::blocking::Client) -> bool {
    client
        .get("http://localhost:11434/api/tags")
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Percorso dell'eseguibile Ollama. Override: BLASKUI_OLLAMA.
fn ollama_exe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("BLASKUI_OLLAMA") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    let candidates = [
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("Programs").join("Ollama").join("ollama.exe")),
        std::env::var_os("USERPROFILE").map(|p| {
            PathBuf::from(p)
                .join("AppData")
                .join("Local")
                .join("Programs")
                .join("Ollama")
                .join("ollama.exe")
        }),
    ];
    candidates.into_iter().flatten().find(|p| p.exists())
}

/// Avvia Ollama in background se non è già in ascolto su :11434.
fn spawn_ollama(app: &tauri::AppHandle) {
    if ollama_available(&reqwest::blocking::Client::new()) {
        eprintln!("[blaskui] Ollama già attivo su :11434");
        return;
    }
    let Some(exe) = ollama_exe() else {
        eprintln!("[blaskui] Ollama non trovato, salto autostart.");
        return;
    };
    eprintln!("[blaskui] avvio Ollama (serve)");
    let mut cmd = Command::new(&exe);
    cmd.arg("serve")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    match cmd.spawn() {
        Ok(mut child) => {
            pipe_logs(child.stdout.take(), "ollama");
            pipe_logs(child.stderr.take(), "ollama-err");
            app.state::<OllamaState>().0.lock().unwrap().replace(child);
        }
        Err(e) => eprintln!("[blaskui] avvio Ollama fallito: {e}"),
    }
}

fn boot(app: tauri::AppHandle) {
    set_progress(&app, 2);
    set_status(&app, "Ricerca porta libera…");
    let installed = is_installed(&app);

    // In una installazione all'avvio estraiamo il payload (backend + venv +
    // build) dalle risorse nella cartella runtime, poi usiamo quelli.
    if installed {
        set_progress(&app, 5);
        if let Err(e) = extract_payload(&app, |msg| set_status(&app, msg)) {
            set_status(&app, &format!("ERRORE estrazione payload: {e}"));
            return;
        }
    }

    let root = repo_root(&app);
    let backend_dir = root.join("backend");
    let python = backend_python(&root);
    let port = find_free_port();

    // In una installazione le dirs del backend stanno nella runtime quanto
    // in una cartella protetta (es. resources read-only): spostiamo i dati
    // utente in %LOCALAPPDATA%\BlaskUI\data. In dev restano quelli del repo.
    let data_dir = if installed {
        let base = std::env::var("BLASKUI_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let local = std::env::var("LOCALAPPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| std::env::temp_dir());
                local.join("BlaskUI").join("data")
            });
        std::fs::create_dir_all(&base).ok();
        base
    } else {
        PathBuf::new()
    };

    if !python.exists() {
        set_status(
            &app,
            &format!(
                "ERRORE: backend Python non trovato ({}). Imposta BLASKUI_BACKEND_PYTHON.",
                python.display()
            ),
        );
        return;
    }

    set_progress(&app, 15);
    set_status(
        &app,
        "Avvio backend BlaskUI… (il primo avvio può richiedere minuti)",
    );
    // La secret key va nella dir scrivibile: data_dir in installazione,
    // backend_dir in dev.
    let key_dir = if installed { &data_dir } else { &backend_dir };
    let _ = std::fs::create_dir_all(key_dir);
    let key = ensure_secret_key(key_dir);

    let mut cmd = Command::new(&python);
    cmd.arg("-m")
        .arg("uvicorn")
        .arg("open_webui.main:app")
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .arg("--workers")
        .arg("1")
        .current_dir(&backend_dir)
        .env("WEBUI_SECRET_KEY", &key)
        .env("PYTHONIOENCODING", "utf-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    // In installazione: dati utente + frontend brandizzato dalla dir risorse.
    if installed {
        cmd.env("DATA_DIR", &data_dir);
        let frontend = root.join("build");
        if frontend.exists() {
            cmd.env("FRONTEND_BUILD_DIR", &frontend);
        }
        // BlaskUI: default installazione (il runtime non legge il .env di sviluppo).
        // Senza queste, l'app installata torna inglese e ripropone gli update Open WebUI.
        cmd.env("DEFAULT_LOCALE", "it-IT");
        cmd.env("ENABLE_VERSION_UPDATE_CHECK", "false");
        cmd.env("AUDIO_STT_ENGINE", "whisper");
        cmd.env("WHISPER_MODEL", "Systran/faster-whisper-small");
        cmd.env("WHISPER_COMPUTE_TYPE", "int8");
        cmd.env("WHISPER_MULTILINGUAL", "true");
        cmd.env("WHISPER_LANGUAGE", "it");
        cmd.env("WHISPER_VAD_FILTER", "true");
        cmd.env(
            "AUDIO_STT_SUPPORTED_CONTENT_TYPES",
            "audio/flac,audio/mpeg,audio/mp3,audio/mp4,audio/ogg,audio/wav,audio/webm,audio/x-m4a,audio/x-wav",
        );
    }
    if let Ok(data_dir_env) = std::env::var("BLASKUI_DATA_DIR") {
        cmd.env("DATA_DIR", data_dir_env);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW: niente console popup
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            set_status(&app, &format!("ERRORE avvio backend: {e}"));
            return;
        }
    };
    pipe_logs(child.stdout.take(), "out");
    pipe_logs(child.stderr.take(), "err");

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            set_status(&app, &format!("ERRORE client HTTP: {e}"));
            let _ = child.kill();
            return;
        }
    };

    eprintln!("[blaskui] backend lanciato (pid {:?}), porta {port}", child.id());
    let health = format!("http://127.0.0.1:{port}/health");
    let deadline = Instant::now() + Duration::from_secs(300);
    let started = Instant::now();
    let mut last_note = Instant::now() - Duration::from_secs(60);
    let ready = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                set_status(
                    &app,
                    &format!("ERRORE: il backend è uscito subito ({status}). Vedi i log."),
                );
                return;
            }
            Ok(None) => {}
            Err(e) => {
                set_status(&app, &format!("ERRORE controllo backend: {e}"));
                let _ = child.kill();
                return;
            }
        }
        match client.get(&health).send() {
            Ok(r) if r.status().is_success() => break true,
            _ => {
                if Instant::now() >= deadline {
                    break false;
                }
                // progresso stimato: 20s → 40%, poi +1% ogni 2s fino a 90%
                let elapsed = started.elapsed().as_secs();
                let pct = if elapsed < 20 {
                    15 + (elapsed * 25 / 20) as u8
                } else {
                    (40 + (elapsed - 20) / 2).min(90) as u8
                };
                set_progress(&app, pct);
                if last_note.elapsed() >= Duration::from_secs(20) {
                    set_status(
                        &app,
                        &format!(
                            "Backend in avvio… ({}s, primo avvio scarica il modello embedding)",
                            started.elapsed().as_secs()
                        ),
                    );
                    last_note = Instant::now();
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    };

    if !ready {
        set_status(&app, "ERRORE: backend non risponde dopo 5 minuti. Vedi i log.");
        let _ = child.kill();
        let _ = child.wait();
        return;
    }

    set_progress(&app, 92);
    // Registra il figlio: verrà killato alla chiusura della finestra.
    app.state::<BackendState>().0.lock().unwrap().replace(child);
    eprintln!("[blaskui] backend pronto su 127.0.0.1:{port}");

    // ComfyUI in parallelo, NON bloccante: l'app si apre comunque se ComfyUI
    // non è presente o impiega minuti a caricare i modelli (ponytail: il
    // modello Z-Image esiste già su questa macchina -> qui e' un no-op).
    spawn_comfyui(&app);
    spawn_studio(&app);
    spawn_ollama(&app);
    if !ollama_available(&client) {
        set_status(
            &app,
            "Ollama non rilevato su localhost:11434 — l'app si apre comunque, installa Ollama per i modelli locali.",
        );
        std::thread::sleep(Duration::from_secs(4));
    }

    set_progress(&app, 100);
    set_status(&app, "Apertura interfaccia…");
    if let Some(w) = app.get_webview_window("main") {
        let target = format!("http://127.0.0.1:{port}/");
        match target.parse() {
            Ok(url) => {
                if w.navigate(url).is_ok() {
                    eprintln!("[blaskui] finestra navigata su {target}");
                } else {
                    set_status(&app, "ERRORE navigazione finestra.");
                }
            }
            Err(e) => set_status(&app, &format!("URL backend non valido: {e}")),
        }
    }
}

fn kill_backend(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<BackendState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(mut child) = guard.take() {
                eprintln!("[blaskui] chiusura backendâ€¦");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

/// Directory ComfyUI. Override: BLASKUI_COMFYUI_DIR; default %USERPROFILE%\\ComfyUI.
fn comfyui_dir() -> Option<PathBuf> {
    let cand = if let Ok(p) = std::env::var("BLASKUI_COMFYUI_DIR") {
        PathBuf::from(p)
    } else {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)?
            .join("ComfyUI")
    };
    cand.join("main.py").exists().then_some(cand)
}

/// Avvia ComfyUI in parallelo e NON bloccante: l'app si apre subito anche se
/// ComfyUI Ã¨ lento a caricare i modelli. Nessuna attesa health qui.
fn spawn_comfyui(app: &tauri::AppHandle) {
    let Some(dir) = comfyui_dir() else {
        eprintln!("[blaskui] ComfyUI non trovato, salto autostart.");
        return;
    };
    let python = dir.join(".venv").join("Scripts").join("python.exe");
    if !python.exists() {
        eprintln!("[blaskui] venv ComfyUI non trovato: {}", python.display());
        return;
    }
    eprintln!("[blaskui] avvio ComfyUI da {}", dir.display());
    let mut cmd = Command::new(&python);
    cmd.arg("main.py")
        .arg("--port")
        .arg("8188")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    match cmd.spawn() {
        Ok(mut child) => {
            pipe_logs(child.stdout.take(), "comfy");
            pipe_logs(child.stderr.take(), "comfy");
            app.state::<ComfyUIState>().0.lock().unwrap().replace(child);
        }
        Err(e) => eprintln!("[blaskui] avvio ComfyUI fallito: {e}"),
    }
}

/// Chiude ComfyUI (evento Destroyed della finestra).
fn kill_comfyui(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<ComfyUIState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(mut child) = guard.take() {
                eprintln!("[blaskui] chiusura ComfyUI…");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

// ─────────────────────────── Unsloth Studio (modelli locali) ───────────────────────────

/// Percorso dell'eseguibile Studio. Override: BLASKUI_UNSLOTH_STUDIO.
fn studio_exe() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("BLASKUI_UNSLOTH_STUDIO") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|home| home.join(".unsloth").join("studio").join("unsloth_studio").join("Scripts").join("unsloth.exe"))
        .filter(|p| p.exists())
}

/// Prima porta libera nella fascia 8888-8908 usata da Studio.
fn studio_port() -> Option<u16> {
    (8888..=8908).find(|p| TcpListener::bind(("127.0.0.1", *p)).is_ok())
}

/// true quando Studio risponde sulla porta con il suo backend UI.
fn studio_healthy(client: &reqwest::blocking::Client, port: u16) -> bool {
    client
        .get(format!("http://127.0.0.1:{port}/api/health"))
        .send()
        .map(|r| r.text().map(|t| t.contains("Unsloth UI Backend")).unwrap_or(false))
        .unwrap_or(false)
}

/// Generazioni attive su Studio: se >0 l'utente sta inferendo, non è idle.
fn studio_busy(client: &reqwest::blocking::Client, port: u16) -> bool {
    client
        .get(format!("http://127.0.0.1:{port}/api/inference/active-generations"))
        .send()
        .map(|r| {
            let body = r.text().unwrap_or_default();
            // lista JSON non vuota = lavoro in corso; stringhe = errori, ignorati
            let trimmed = body.trim();
            trimmed.starts_with('[') && !trimmed.starts_with("[]")
        })
        .unwrap_or(false)
}

/// Scrive %LOCALAPPDATA%\BlaskUI\unsloth.json con porta e URL /v1 per l'app.
fn write_studio_info(port: u16) {
    if let Some(base) = install_base() {
        let _ = std::fs::write(
            base.join("unsloth.json"),
            format!("{{\"port\": {port}, \"url\": \"http://127.0.0.1:{port}/v1\"}}"),
        );
    }
}

/// Avvia Studio in background e registra la porta per l'app.
/// Se Studio è già acceso (es. avviato a mano), lo adotta: usa la sua porta
/// senza diventarne proprietario (niente kill all'uscita, niente idle-off).
fn spawn_studio(app: &tauri::AppHandle) {
    // 1) adotta un'istanza sana già in ascolto, se c'è
    if let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        for port in 8888..=8908 {
            if studio_healthy(&client, port) {
                eprintln!("[blaskui] Unsloth Studio già attivo su :{port}, lo adotto");
                let state = app.state::<StudioState>();
                *state.port.lock().unwrap() = Some(port);
                *state.last_active.lock().unwrap() = Instant::now();
                write_studio_info(port);
                return;
            }
        }
    }
    // 2) altrimenti lancialo da zero
    let Some(exe) = studio_exe() else {
        eprintln!("[blaskui] Unsloth Studio non trovato, salto autostart.");
        return;
    };
    let Some(port) = studio_port() else {
        eprintln!("[blaskui] nessuna porta libera in 8888-8908 per Studio");
        return;
    };
    eprintln!("[blaskui] avvio Unsloth Studio su 127.0.0.1:{port}");
    let mut cmd = Command::new(&exe);
    cmd.arg("studio")
        .arg("-p")
        .arg(port.to_string())
        .current_dir(
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_default(),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    match cmd.spawn() {
        Ok(mut child) => {
            pipe_logs(child.stdout.take(), "studio");
            pipe_logs(child.stderr.take(), "studio-err");
            let state = app.state::<StudioState>();
            *state.child.lock().unwrap() = Some(child);
            *state.port.lock().unwrap() = Some(port);
            *state.last_active.lock().unwrap() = Instant::now();
            write_studio_info(port);
            watch_studio_idle(app.clone());
        }
        Err(e) => eprintln!("[blaskui] avvio Studio fallito: {e}"),
    }
}

/// Thread di guardia: se Studio resta inattivo troppo, lo spegne (si riaccende al
/// prossimo avvio di BlaskUI). ponytail: due segnali semplici (focus finestra +
/// generazioni attive); niente euristiche complicate.
fn watch_studio_idle(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let Some(state) = app.try_state::<StudioState>() else {
                return;
            };
            let Some(port) = *state.port.lock().unwrap() else {
                return; // già spento
            };
            if !studio_healthy(&client, port) {
                *state.port.lock().unwrap() = None;
                if let Ok(mut guard) = state.child.lock() {
                    if let Some(mut c) = guard.take() {
                        let _ = c.kill();
                    }
                }
                return;
            }
            if studio_busy(&client, port) {
                *state.last_active.lock().unwrap() = Instant::now();
                continue;
            }
            let idle = state.last_active.lock().unwrap().elapsed();
            if idle >= Duration::from_secs(STUDIO_IDLE_MINUTES * 60) {
                eprintln!("[blaskui] Studio inattivo da {:?}: chiusura", idle);
                *state.port.lock().unwrap() = None;
                if let Ok(mut guard) = state.child.lock() {
                    if let Some(mut c) = guard.take() {
                        let _ = c.kill();
                        let _ = c.wait();
                    }
                }
                return;
            }
        }
    });
}

/// Segna attività utente (focus finestra) per il timer idle di Studio.
fn touch_studio(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<StudioState>() {
        if state.port.lock().unwrap().is_some() {
            *state.last_active.lock().unwrap() = Instant::now();
        }
    }
}

fn kill_studio(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<StudioState>() {
        *state.port.lock().unwrap() = None;
        if let Ok(mut guard) = state.child.lock() {
            if let Some(mut child) = guard.take() {
                eprintln!("[blaskui] chiusura Unsloth Studio…");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn kill_ollama(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<OllamaState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(mut child) = guard.take() {
                eprintln!("[blaskui] chiusura Ollama…");
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}
fn main() {
    tauri::Builder::default()
        .manage(BackendState(Mutex::new(None)))
        .manage(ComfyUIState(Mutex::new(None)))
        .manage(OllamaState(Mutex::new(None)))
        .manage(StudioState {
            child: Mutex::new(None),
            port: Mutex::new(None),
            last_active: Mutex::new(Instant::now()),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || boot(handle));
            Ok(())
        })
        .on_window_event(|window, event| {
            let app = window.app_handle().clone();
            if window.label() == "main" {
                match event {
                    WindowEvent::Destroyed => {
                        kill_backend(&app);
                        kill_comfyui(&app);
                        kill_studio(&app);
                        kill_ollama(&app);
                    }
                    WindowEvent::Focused(_) => touch_studio(&app),
                    _ => {}
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("blaskui: errore avvio runtime Tauri");
}

