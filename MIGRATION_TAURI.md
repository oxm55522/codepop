# Codepop → Tauri 마이그레이션 플랜

> ## 진행 상황 (2026-06-05)
> - ✅ **Phase 0** 스캐폴딩 — Rust 설치, Tauri v2 통합, 투명 윈도우 기동
> - ✅ **Phase 1** config/cache/gemini Rust 포팅 — 기존 Lua 캐시로 해시 동일성 교차검증(18/18) + 라이브 Gemini 호출 검증
> - ✅ **Phase 2** popup.html 이식 + CDN 4종 로컬 번들(오프라인) + ipc.js IPC 어댑터 — 헤드리스 브라우저로 렌더 전수 검증
> - ✅ **Phase 3** 글로벌 단축키 + 선택 캡처 — 동작 확인(GUI+권한). invoke 경로 버그 수정(버튼이 죽은 webkit 슬롯 대신 `__TAURI__.core.invoke` 직접 호출)
> - ✅ **Phase 4** 설정창(API키/모델/단축키/언어) + 메뉴바 트레이 + 온보딩(키 없으면 설정 자동) + 앱 재클릭→설정 + 동적 단축키
> - 🔶 **Phase 5** `.app` + `.dmg` 빌드 완료(ad-hoc). 정식 배포용 Apple 서명·공증은 미적용($99)
>
> ### Phase 3 수동 테스트 (GUI + 권한 필요)
> 1. `cd codepop && PATH="$HOME/.cargo/bin:$PATH" npm run tauri dev`
> 2. 아무 앱에서 코드를 드래그 선택 → `⌘⇧E`
> 3. 첫 실행 시 **손쉬운 사용 권한** 다이얼로그 → 시스템 설정에서 허용 후 재시도
> 4. 커서 옆에 팝업 → 로딩 → Gemini 설명 표시 확인
> 5. ESC 닫기 / 🔄 재분석 / 한↔영 토글 동작 확인
>
> ⚠️ dev 바이너리는 ad-hoc 서명이라 재빌드 시 권한 재요청될 수 있음(Phase 5 .app 서명으로 해결).



> Hammerspoon 의존 제거 → 누구나 `.app` 받아 쓰는 데스크탑 앱.
> $99 Apple Developer 없이 무료 배포(ad-hoc 서명 + 첫 실행 우클릭 열기).

---

## 0. 목표 / 비목표

**목표**
- Hammerspoon 설치 없이 `.dmg` 다운 → 드래그 → `⌘⇧E` 동작
- 기존 `popup.html` UI/UX 100% 보존
- 용량 5~10MB, "0원" 컨셉 유지
- 오프라인 동작(CDN 4종 로컬 번들)

**비목표(이번엔 안 함)**
- Windows/Linux 크로스 플랫폼 (1차는 macOS만)
- $99 공증 — 무료 배포는 첫 실행 "우클릭 → 열기" 안내로 우회
- 자동 업데이트(추후 tauri-plugin-updater)

---

## 1. 책임 매핑 (Hammerspoon → Tauri)

| 현재 (Lua/HS) | 이전처 | 난이도 | 비고 |
|---|---|---|---|
| `hs.hotkey` 글로벌 단축키 | `tauri-plugin-global-shortcut` | ★ | 거의 그대로 |
| `getSelectedText()` 클립보드 트릭 | Rust `selection.rs` (enigo + NSPasteboard) | ★★★ | **핵심 난관**. Accessibility 권한 |
| `hs.webview` 커서 옆 팝업 | Tauri Window (transparent/always-on-top/no-deco) | ★★ | macOS는 WKWebView라 popup.html 그대로 렌더 |
| `usercontent` JS→Lua | `invoke()` (JS→Rust command) | ★ | 어댑터로 흡수 |
| `evaluateJavaScript` Lua→JS | Tauri `emit`/`listen` 이벤트 | ★ | render/loading/error 3종 |
| `callGemini()` | Rust `gemini.rs` (reqwest) | ★ | 재시도/타임아웃 로직 포팅 |
| SHA256 캐시 + LRU | Rust `cache.rs` (sha2) | ★ | `~/.codepop/cache/` 그대로 |
| `loadConfig()` | Rust `config.rs` (serde_json) | ★ | `~/.codepop/config.json` 그대로 |
| ESC 닫기 | JS keydown 또는 window focus 단축키 | ★ | |

> `~/.codepop/` 경로/포맷은 **그대로 유지** → 기존 HS 사용자도 config 재입력 불필요.

---

## 2. 타깃 구조

```
codepop/
├── hammerspoon/              # 레거시. 당분간 보존(롤백용), 안정화 후 제거
├── src/                      # 프론트엔드 (popup.html 이식)
│   ├── index.html           # popup.html 기반 (CDN → 로컬 vendor)
│   ├── ipc.js               # ★ IPC 어댑터 (webkit→tauri). 신규 ~40줄
│   └── vendor/              # marked, dompurify, highlight.js, pretendard (오프라인)
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json      # 윈도우/번들/권한 설정
│   ├── build.rs
│   ├── icons/
│   ├── capabilities/        # 권한 정의(global-shortcut 등)
│   └── src/
│       ├── main.rs          # 부트스트랩, 트레이, 단축키 등록
│       ├── selection.rs     # ★ getSelectedText 네이티브 포팅
│       ├── gemini.rs        # callGemini 포팅
│       ├── cache.rs         # 캐시 R/W + prune
│       ├── config.rs        # config 로드/저장
│       └── popup.rs         # 커서 위치 + 윈도우 생성/표시/숨김
├── MIGRATION_TAURI.md       # (이 문서)
└── README.md / CLAUDE.md    # 추후 갱신
```

---

## 3. 함수 단위 포팅 표

### codepop.lua → Rust

| Lua 함수 | → | 위치 |
|---|---|---|
| `loadConfig` | `config::load()` | config.rs |
| `getCached/setCached/pruneCache/invalidateCache/touchCache` | `cache::*` | cache.rs |
| `hashText` (SHA256 + PROMPT_VERSION + lang) | `cache::hash_text()` | cache.rs |
| `getSelectedText` (백업→clear→⌘C→폴링→복원) | `selection::get_selected_text()` | selection.rs |
| `withLineNumbers` | `gemini::with_line_numbers()` | gemini.rs |
| `callGemini` (프롬프트/재시도/타임아웃/응답검증) | `gemini::call()` | gemini.rs |
| `reasonKr` | `gemini::reason_kr()` | gemini.rs |
| `runAnalysis` | `commands::analyze()` (Tauri command) | main.rs |
| `M.explain` | 단축키 핸들러 → `analyze(use_cache=true)` | main.rs |
| `M.retry` / `M.switchLanguage` | `#[command] retry()` / `switch_lang()` | main.rs |
| 팝업 생성/표시/갱신 | `popup::*` + emit 이벤트 | popup.rs |

### popup.html (거의 유지, 접점만 교체)

| 현재 | 교체 |
|---|---|
| `sendMessage(type)` → `webkit.messageHandlers.codepop.postMessage` | `invoke(type)` (ipc.js) |
| `window.__CODEPOP_DATA__` 주입 | 최초 `invoke('get_initial')` 또는 `listen('render')` |
| Lua가 `__codepopRender/__codepopLoading/__codepopError` 호출 | Rust `emit('render'|'loading'|'error')` → JS `listen` 후 동일 함수 호출 |

→ **`__codepopRender/Loading/Error` 함수 본문은 그대로.** ipc.js가 이벤트를 받아 이 함수들을 호출만 함.

---

## 4. 단계별 실행 (각 단계 = 독립 검증 가능)

### Phase 0 — 스캐폴딩
- `create-tauri-app` (vanilla, no framework)
- 빈 투명 윈도우 1개 띄우기 확인
- **검증:** `cargo tauri dev` 로 창 뜸

### Phase 1 — 순수 로직 포팅 (UI/단축키 없이)
- `config.rs`, `cache.rs`, `gemini.rs` 작성
- 임시 `#[command] analyze_text(text)` 로 노출 → 더미 버튼으로 호출
- **검증:** 코드 문자열 넣으면 Gemini 응답 JSON 반환 + 캐시 파일 생성 확인
- 이 단계는 네이티브/권한 없이 끝나서 **포팅 정확성 먼저 확보**

### Phase 2 — 팝업 + UI 이식
- `popup.html` → `src/index.html` 복사, CDN→`vendor/` 로컬화
- `ipc.js` 어댑터 작성 (invoke/listen ↔ 기존 함수)
- `popup.rs`: 커서 좌표(`NSEvent.mouseLocation` 또는 device_query) 기준 윈도우 배치 + 화면 경계 클램프(Lua `popupFrame` 로직 포팅)
- Phase 1 더미 데이터로 렌더/스피너/에러/언어토글/주석/splitter 전부 동작 확인
- **검증:** 단축키 없이도 팝업 UI 완전 동작

### Phase 3 — 글로벌 단축키 + 선택 캡처 (★ 끝판왕)
- `tauri-plugin-global-shortcut` 로 `⌘⇧E` 등록 → `analyze()` 호출
- `selection.rs`:
  - 클립보드 백업(`NSPasteboard` 전체 타입) → clear → `enigo`로 ⌘C → `changeCount` 폴링(500ms) → 읽기 → 복원
  - Lua `getSelectedText`의 가드(changeCount 검증, 길이 5~20000) 동일 이식
- `AXIsProcessTrustedWithOptions`로 Accessibility 권한 프롬프트
- **검증:** 브라우저/VSCode/터미널/Slack에서 드래그→⌘⇧E→팝업

### Phase 4 — 마무리
- 트레이 아이콘(메뉴: 권한 안내, 종료, config 열기)
- ESC 닫기, 언어 토글 config 영구 저장(`switch_lang`이 config.json write)
- 첫 실행 온보딩(권한 안내 화면)
- `~/.codepop/config.json` 없을 때 API 키 입력 UI

### Phase 5 — 패키징 / 배포
- `tauri.conf.json` 번들 설정, 아이콘
- **ad-hoc 서명**(`codesign -s -`) + DMG 생성
- README에 "확인되지 않은 개발자 → 우클릭 열기" + "손쉬운 사용 권한 켜기" 안내
- 기존 `install.sh`/`uninstall.sh` 는 HS 전용 → 보존 또는 DMG 링크로 대체

---

## 5. 리스크 / 주의

| 리스크 | 영향 | 대응 |
|---|---|---|
| **선택 캡처 네이티브 포팅** | 가장 까다로움. enigo/NSPasteboard objc 바인딩 | Phase 3 단독 집중. 안 되면 AX API(`AXSelectedText`) 폴백 검토(단 일부 앱 미지원) |
| **Accessibility 권한 + TCC** | ad-hoc 서명은 **재빌드마다 권한 리셋** 가능(서명/경로 기반) | 개발 중 불편 감수. 배포 .app은 경로 고정이라 사용자 1회로 끝 |
| **무료 배포 Gatekeeper** | "확인 안 됨" 경고 | 우클릭 열기 안내. 추후 $99 시 공증으로 제거 |
| **CDN 오프라인** | 패키지 앱이 CDN 의존하면 비행기모드 깨짐 | vendor 로컬 번들 (Phase 2) |
| **클립보드 클로버** | ⌘C 트릭이 사용자 클립보드 덮음 | Lua처럼 readAll/writeAll 전체 백업·복원 포팅 |
| **단축키 충돌** | `⌘⇧E` 타 앱과 겹침 | 설정에서 변경 가능하게(추후) |

---

## 6. 기존 의사결정 보존 (CLAUDE.md 트레이드오프)

마이그레이션 중 **재논의 금지** — 이미 검토됨:
- SHA256 정확 일치 캐시(정규화 X)
- 라인 번호 LLM 직접 카운트 X (`withLineNumbers`)
- 클립보드 changeCount 폴링(고정 sleep X) — Electron 앱 대응
- `~/.codepop/` 평문 + chmod 600
- DOMPurify 화이트리스트 XSS 방어
- 20,000자 입력 한도

→ Rust 포팅 시 위 동작/상수 **그대로 이식**.

---

## 7. 권장 크레이트

| 용도 | 크레이트 |
|---|---|
| Tauri | `tauri` v2 |
| 글로벌 단축키 | `tauri-plugin-global-shortcut` |
| HTTP | `reqwest` (rustls) |
| JSON | `serde` / `serde_json` |
| SHA256 | `sha2` |
| 키 입력 시뮬(⌘C) | `enigo` |
| NSPasteboard/AX/커서좌표 | `objc2` + `objc2-app-kit` (또는 `cocoa`/`core-graphics`) |
| 마우스 좌표 | `objc2-app-kit`(NSEvent) 또는 `device_query` |

---

## 8. 첫 PR 권장 범위

Phase 0 + Phase 1 (스캐폴딩 + 순수 로직).
네이티브/권한 없이 "코드 문자열 → Gemini 응답 + 캐시"까지 검증되면 절반은 끝난 것.
Phase 3(선택 캡처)는 별도 PR로 격리 — 디버깅 길어질 수 있음.
</content>
</invoke>
