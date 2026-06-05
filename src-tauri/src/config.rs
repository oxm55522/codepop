// config.rs — ~/.codepop/config.json 로드/저장
// Lua: loadConfig() / M.switchLanguage 의 config write 포팅
// 경로/포맷은 Hammerspoon 버전과 동일하게 유지 (기존 사용자 재입력 불필요)

use serde::Deserialize;
use std::path::PathBuf;

pub const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
pub const DEFAULT_SHORTCUT: &str = "cmd+shift+e";

pub fn codepop_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_default().join(".codepop")
}

/// 시스템 언어가 한국어면 ko, 아니면 en (신규 사용자 기본값)
pub fn system_default_lang() -> String {
    match sys_locale::get_locale() {
        Some(l) if l.to_lowercase().starts_with("ko") => "ko".to_string(),
        _ => "en".to_string(),
    }
}

pub fn config_path() -> PathBuf {
    codepop_dir().join("config.json")
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(rename = "apiKey", default)]
    pub api_key: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub shortcut: Option<String>,
}

impl Config {
    pub fn model(&self) -> String {
        self.model
            .clone()
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string())
    }

    /// config.language 가 "ko"/"en" 일 때만 채택, 미설정이면 시스템 언어로 추정
    pub fn lang(&self) -> String {
        match self.language.as_deref() {
            Some("en") => "en".to_string(),
            Some("ko") => "ko".to_string(),
            _ => system_default_lang(),
        }
    }

    pub fn shortcut(&self) -> String {
        self.shortcut
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_SHORTCUT.to_string())
    }
}

/// 설정 화면에서 전체 저장. config 디렉터리 생성 + chmod 600 (API 키 보호).
pub fn save_settings(api_key: &str, model: &str, language: &str, shortcut: &str) -> Result<(), String> {
    let dir = codepop_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let val = serde_json::json!({
        "apiKey": api_key,
        "model": model,
        "language": language,
        "shortcut": shortcut,
    });
    let out = serde_json::to_string_pretty(&val).map_err(|e| e.to_string())?;
    let path = config_path();
    std::fs::write(&path, out).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// 로드 실패/미존재/파싱오류는 Err(메시지) — 호출부가 사용자에게 안내
pub fn load() -> Result<Config, String> {
    let path = config_path();
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| format!("{} 없음", path.display()))?;
    serde_json::from_str::<Config>(&raw).map_err(|_| "config.json 파싱 실패".to_string())
}

/// 언어 토글 시 config.language 만 갱신 (다른 필드 보존).
/// Lua 처럼 raw JSON 을 읽어 language 키만 덮어쓰고 다시 씀.
#[allow(dead_code)] // Phase 4: 언어 토글에서 사용
pub fn save_language(lang: &str) -> Result<(), String> {
    let path = config_path();
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut val: serde_json::Value =
        serde_json::from_str(&raw).map_err(|_| "config.json 파싱 실패".to_string())?;
    if let Some(obj) = val.as_object_mut() {
        obj.insert("language".into(), serde_json::Value::String(lang.to_string()));
    }
    let out = serde_json::to_string_pretty(&val).map_err(|e| e.to_string())?;
    std::fs::write(&path, out).map_err(|e| e.to_string())
}
