// cache.rs — SHA256 정확일치 캐시 + LRU(500) prune
// Lua: hashText / getCached / setCached / pruneCache / invalidateCache / touchCache 포팅
// 저장 위치/포맷/해시 알고리즘은 Hammerspoon 버전과 100% 동일 (기존 캐시 재사용 가능)

use crate::config;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

pub const PROMPT_VERSION: &str = "v6"; // 프롬프트 형식 bump 시 캐시 자동 무효
const MAX_CACHE_ENTRIES: usize = 500;

pub fn cache_dir() -> PathBuf {
    config::codepop_dir().join("cache")
}

fn entry_path(hash: &str) -> PathBuf {
    cache_dir().join(format!("{hash}.json"))
}

/// Lua: SHA256(text .. ":" .. PROMPT_VERSION .. ":" .. currentLang)
pub fn hash_text(text: &str, lang: &str) -> String {
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    h.update(b":");
    h.update(PROMPT_VERSION.as_bytes());
    h.update(b":");
    h.update(lang.as_bytes());
    hex::encode(h.finalize())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub explanation: String,
    pub code: String,
    pub ts: i64,
}

pub fn get(hash: &str) -> Option<CacheEntry> {
    let raw = fs::read_to_string(entry_path(hash)).ok()?;
    serde_json::from_str::<CacheEntry>(&raw).ok()
}

pub fn set(hash: &str, entry: &CacheEntry) {
    let _ = fs::create_dir_all(cache_dir());
    if let Ok(json) = serde_json::to_string(entry) {
        let _ = fs::write(entry_path(hash), json);
    }
    prune();
}

#[allow(dead_code)] // Phase 3: 재분석(M.retry) 에서 사용
pub fn invalidate(hash: &str) {
    let _ = fs::remove_file(entry_path(hash));
}

/// 캐시 적중 시 mtime 갱신용 (LRU 신선도 유지). 내용 재기록으로 mtime bump.
pub fn touch(hash: &str) {
    let p = entry_path(hash);
    if let Ok(bytes) = fs::read(&p) {
        let _ = fs::write(&p, bytes);
    }
}

/// 500개 초과 시 mtime 오래된 것부터 삭제
fn prune() {
    let dir = cache_dir();
    let mut entries: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let Ok(read) = fs::read_dir(&dir) else { return };
    for e in read.flatten() {
        let path = e.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(meta) = e.metadata() {
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            entries.push((path, mtime));
        }
    }
    if entries.len() <= MAX_CACHE_ENTRIES {
        return;
    }
    entries.sort_by_key(|(_, m)| *m); // 오래된 것 먼저
    let remove_count = entries.len() - MAX_CACHE_ENTRIES;
    for (path, _) in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        let a = hash_text("hello", "ko");
        let b = hash_text("hello", "ko");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn hash_varies_by_lang() {
        assert_ne!(hash_text("x", "ko"), hash_text("x", "en"));
    }

    /// Lua(Hammerspoon) 가 만든 기존 캐시 파일로 해시 알고리즘 동일성 검증.
    /// - 현재 포맷(v6 + ko/en): Rust hash_text 가 그대로 재현해야 함
    /// - 그 외: lang/version 추가 전 레거시 포맷(plain / code:vN) 으로만 설명되어야 함
    ///   → 설명 안 되는 파일이 있으면 알고리즘 버그 또는 데이터 손상.
    /// 환경 의존(~/.codepop/cache) 이라 기본 무시 — `cargo test --ignored -- --nocapture`.
    #[test]
    #[ignore]
    fn cross_validate_against_lua_cache() {
        use sha2::{Digest, Sha256};
        let sha = |s: &str| -> String {
            let mut h = Sha256::new();
            h.update(s.as_bytes());
            hex::encode(h.finalize())
        };
        // lang 도입 전 구포맷들 (확인된 레거시: plain, code:v4, code:v5)
        let legacy = |code: &str, stem: &str| -> bool {
            if sha(code) == stem {
                return true;
            }
            (0..=7).any(|v| sha(&format!("{code}:v{v}")) == stem)
        };

        let dir = cache_dir();
        let read = fs::read_dir(&dir).expect("캐시 디렉터리 없음");
        let (mut checked, mut current, mut legacy_cnt) = (0, 0, 0);
        let mut unexplained = Vec::new();
        for e in read.flatten() {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = path.file_stem().unwrap().to_str().unwrap().to_string();
            let Ok(raw) = fs::read_to_string(&path) else { continue };
            let Ok(entry) = serde_json::from_str::<CacheEntry>(&raw) else { continue };
            checked += 1;
            if hash_text(&entry.code, "ko") == stem || hash_text(&entry.code, "en") == stem {
                current += 1; // ★ 현재 알고리즘으로 재현됨
            } else if legacy(&entry.code, &stem) {
                legacy_cnt += 1;
            } else {
                unexplained.push(stem[..12].to_string());
            }
        }
        eprintln!("교차검증 {checked}개: 현재포맷(v6) {current} / 레거시 {legacy_cnt} / 미설명 {}", unexplained.len());
        assert!(checked > 0, "검증할 캐시 파일이 없음");
        assert!(current > 0, "현재 포맷으로 재현되는 캐시가 하나도 없음 — 알고리즘 의심");
        assert!(unexplained.is_empty(), "설명 안 되는 캐시(버그/손상): {unexplained:?}");
    }
}
