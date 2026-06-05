// Codepop — Tauri 백엔드 진입점
// Phase 4: 설정창(API 키/모델/단축키/언어) + 트레이 + 온보딩 + 동적 단축키.

mod cache;
mod config;
mod gemini;
#[cfg(target_os = "macos")]
mod popup;
#[cfg(target_os = "macos")]
mod selection;

use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};

/// 간단 i18n — 영어 모드일 때 백엔드 메시지도 영어로
fn t(lang: &str, ko: &str, en: &str) -> String {
    if lang == "en" { en.to_string() } else { ko.to_string() }
}

#[derive(Default)]
struct AppState {
    last_text: Mutex<Option<String>>,
    shortcut: Mutex<Option<tauri_plugin_global_shortcut::Shortcut>>,
}

// ───────────────────────── 분석 흐름 ─────────────────────────

/// 공통 분석 흐름 (Lua runAnalysis). 결과를 이벤트로 팝업에 전달.
async fn run_analysis(app: &AppHandle, text: String, use_cache: bool) {
    let cfg = match config::load() {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("error", serde_json::json!({ "error": e, "lang": "ko" }));
            return;
        }
    };
    let lang = cfg.lang();

    let _ = app.emit("loading", serde_json::json!({ "code": text, "lang": lang }));

    if cfg.api_key.is_empty() {
        let _ = app.emit(
            "error",
            serde_json::json!({
                "error": t(&lang,
                    "API 키가 없습니다. 메뉴바 ✦ → 설정 에서 키를 등록하세요.",
                    "No API key. Open the menu bar ✦ → Settings to add your key."),
                "lang": lang,
            }),
        );
        return;
    }

    let hash = cache::hash_text(&text, &lang);

    if use_cache {
        if let Some(entry) = cache::get(&hash) {
            cache::touch(&hash);
            let _ = app.emit(
                "render",
                serde_json::json!({
                    "code": text, "explain": entry.explanation,
                    "source": "cache", "elapsed": 0, "lang": lang
                }),
            );
            return;
        }
    }

    let start = Instant::now();
    match gemini::call(&text, &cfg, &lang).await {
        Ok(explanation) => {
            let elapsed = start.elapsed().as_millis() as u64;
            cache::set(
                &hash,
                &cache::CacheEntry {
                    explanation: explanation.clone(),
                    code: text.clone(),
                    ts: 0,
                },
            );
            let _ = app.emit(
                "render",
                serde_json::json!({
                    "code": text, "explain": explanation,
                    "source": "Gemini", "elapsed": elapsed, "lang": lang
                }),
            );
        }
        Err(e) => {
            let _ = app.emit("error", serde_json::json!({ "error": e, "lang": lang }));
        }
    }
}

#[cfg(target_os = "macos")]
fn show_error(app: &AppHandle, lang: &str, msg: &str) {
    popup::show_at_cursor(app);
    let _ = app.emit("error", serde_json::json!({ "error": msg, "lang": lang }));
}

/// 핫키 핸들러 — Lua M.explain
#[cfg(target_os = "macos")]
fn on_hotkey(app: &AppHandle) {
    let lang = config::load().map(|c| c.lang()).unwrap_or_else(|_| "ko".into());

    if !selection::accessibility_trusted(false) {
        selection::accessibility_trusted(true);
        show_error(
            app,
            &lang,
            &t(&lang,
                "손쉬운 사용 권한이 필요합니다.\n시스템 설정 → 개인정보 보호 및 보안 → 손쉬운 사용\n에서 Codepop 을 허용한 뒤 다시 시도하세요.",
                "Accessibility permission is required.\nSystem Settings → Privacy & Security → Accessibility\nEnable Codepop, then try again."),
        );
        return;
    }

    let text = match selection::get_selected_text() {
        Some(t) => t,
        None => {
            show_error(app, &lang, &t(&lang, "선택된 텍스트가 없거나 너무 짧음", "No text selected, or too short"));
            return;
        }
    };

    let n = text.chars().count();
    if n < 5 {
        show_error(app, &lang, &t(&lang, "선택된 텍스트가 없거나 너무 짧음", "No text selected, or too short"));
        return;
    }
    if n > 20000 {
        show_error(app, &lang, &t(&lang, "선택이 너무 김 (20000자 초과)", "Selection too long (over 20,000 chars)"));
        return;
    }

    if let Some(state) = app.try_state::<AppState>() {
        *state.last_text.lock().unwrap() = Some(text.clone());
    }
    popup::show_at_cursor(app);

    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        run_analysis(&app2, text, true).await;
    });
}

// ───────────────────────── 단축키 ─────────────────────────

#[cfg(target_os = "macos")]
fn key_to_code(k: &str) -> Option<tauri_plugin_global_shortcut::Code> {
    use tauri_plugin_global_shortcut::Code::*;
    Some(match k {
        "a" => KeyA, "b" => KeyB, "c" => KeyC, "d" => KeyD, "e" => KeyE,
        "f" => KeyF, "g" => KeyG, "h" => KeyH, "i" => KeyI, "j" => KeyJ,
        "k" => KeyK, "l" => KeyL, "m" => KeyM, "n" => KeyN, "o" => KeyO,
        "p" => KeyP, "q" => KeyQ, "r" => KeyR, "s" => KeyS, "t" => KeyT,
        "u" => KeyU, "v" => KeyV, "w" => KeyW, "x" => KeyX, "y" => KeyY, "z" => KeyZ,
        "0" => Digit0, "1" => Digit1, "2" => Digit2, "3" => Digit3, "4" => Digit4,
        "5" => Digit5, "6" => Digit6, "7" => Digit7, "8" => Digit8, "9" => Digit9,
        "space" => Space, "enter" | "return" => Enter,
        _ => return None,
    })
}

/// "cmd+shift+e" → Shortcut
#[cfg(target_os = "macos")]
fn parse_shortcut(s: &str) -> Option<tauri_plugin_global_shortcut::Shortcut> {
    use tauri_plugin_global_shortcut::{Modifiers, Shortcut};
    let mut mods = Modifiers::empty();
    let mut code = None;
    for part in s.split('+') {
        match part.trim().to_lowercase().as_str() {
            "cmd" | "command" | "super" | "meta" | "win" => mods |= Modifiers::SUPER,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" | "opt" => mods |= Modifiers::ALT,
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            other => code = key_to_code(other),
        }
    }
    let code = code?;
    let mods = if mods.is_empty() { None } else { Some(mods) };
    Some(Shortcut::new(mods, code))
}

/// 단축키 적용 — 기존 등록 해제 후 새로 등록, 상태에 보관
#[cfg(target_os = "macos")]
fn apply_shortcut(app: &AppHandle, s: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let new = parse_shortcut(s).ok_or_else(|| format!("단축키 형식 오류: {s}"))?;
    let gs = app.global_shortcut();
    if let Some(state) = app.try_state::<AppState>() {
        if let Some(old) = state.shortcut.lock().unwrap().take() {
            let _ = gs.unregister(old);
        }
    }
    gs.register(new).map_err(|e| e.to_string())?;
    if let Some(state) = app.try_state::<AppState>() {
        *state.shortcut.lock().unwrap() = Some(new);
    }
    Ok(())
}

// ───────────────────────── 설정창 ─────────────────────────

fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("settings.html".into()),
    )
    .title("Codepop 설정")
    .inner_size(440.0, 560.0)
    .resizable(false)
    .build();
}

// ───────────────────────── 커맨드 ─────────────────────────

#[tauri::command]
fn load_settings() -> serde_json::Value {
    let c = config::load().unwrap_or_default();
    serde_json::json!({
        "apiKey": c.api_key,
        "model": c.model(),
        "language": c.lang(),
        "shortcut": c.shortcut(),
    })
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    api_key: String,
    model: String,
    language: String,
    shortcut: String,
) -> Result<(), String> {
    config::save_settings(&api_key, &model, &language, &shortcut)?;
    #[cfg(target_os = "macos")]
    {
        apply_shortcut(&app, &shortcut)?;
    }
    let _ = &app;
    Ok(())
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) {
    use tauri_plugin_opener::OpenerExt;
    let _ = app.opener().open_url(url, None::<&str>);
}

#[tauri::command]
fn open_settings_cmd(app: AppHandle) {
    open_settings(&app);
}

/// dev 패널 — 코드 직접 입력 (선택 캡처 안 되는 환경/테스트)
#[tauri::command]
async fn dev_analyze(app: AppHandle, state: State<'_, AppState>, text: String) -> Result<(), String> {
    *state.last_text.lock().unwrap() = Some(text.clone());
    run_analysis(&app, text, true).await;
    Ok(())
}

/// Lua M.retry — 캐시 무효화 후 재분석
#[tauri::command]
async fn retry(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let text = state.last_text.lock().unwrap().clone();
    let Some(text) = text else { return Ok(()) };
    let lang = config::load().map(|c| c.lang()).unwrap_or_else(|_| "ko".into());
    cache::invalidate(&cache::hash_text(&text, &lang));
    run_analysis(&app, text, false).await;
    Ok(())
}

/// Lua M.switchLanguage — 언어 토글 + config 저장 + 재분석
#[tauri::command]
async fn switch_lang(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let text = state.last_text.lock().unwrap().clone();
    let Some(text) = text else { return Ok(()) };
    let cur = config::load().map(|c| c.lang()).unwrap_or_else(|_| "ko".into());
    let next = if cur == "en" { "ko" } else { "en" };
    let _ = config::save_language(next);
    run_analysis(&app, text, true).await;
    Ok(())
}

#[tauri::command]
fn close_popup(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

// ───────────────────────── 트레이 ─────────────────────────

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;

    let open_i = MenuItem::with_id(app, "open_settings", "설정", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &quit_i])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Codepop — ⌘⇧E 로 코드 설명")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_settings" => open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default());

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_handler(|app, _shortcut, event| {
                use tauri_plugin_global_shortcut::ShortcutState;
                if event.state() == ShortcutState::Pressed {
                    on_hotkey(app);
                }
            })
            .build(),
    );

    builder
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            open_url,
            open_settings_cmd,
            dev_analyze,
            retry,
            switch_lang,
            close_popup
        ])
        .setup(|app| {
            let handle = app.handle();
            build_tray(&handle)?;

            #[cfg(target_os = "macos")]
            {
                let sc = config::load()
                    .map(|c| c.shortcut())
                    .unwrap_or_else(|_| config::DEFAULT_SHORTCUT.to_string());
                let _ = apply_shortcut(&handle, &sc);

                // 온보딩: API 키 없으면 설정창 자동 오픈
                let need_onboard = config::load().map(|c| c.api_key.is_empty()).unwrap_or(true);
                if need_onboard {
                    open_settings(&handle);
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            // 앱 재실행/독 클릭 → 설정창
            if let tauri::RunEvent::Reopen { .. } = event {
                open_settings(app);
            }
        });
}
