// ipc.js — Codepop Tauri IPC 어댑터
// popup.html(index.html) 의 내부 로직은 건드리지 않고, Hammerspoon 시절의 두 접점만 교체:
//   1) JS→백엔드:  webkit.messageHandlers.codepop.postMessage  →  Tauri invoke
//   2) 백엔드→JS:  Lua evaluateJavaScript(__codepopRender..)   →  Tauri event listen
// 이 스크립트는 인라인 IIFE(렌더 함수 정의) 다음에 로드되므로 window.__codepop* 가 이미 존재.

(function () {
  const T = window.__TAURI__;

  // ── 1) JS → Rust : ESC 등 ipc.js 자체에서 보내는 커맨드 ──
  // (popup 의 버튼들은 index.html sendMessage 가 직접 invoke 하도록 수정됨)
  const CMD = { 'retry': 'retry', 'switch-lang': 'switch_lang', 'close': 'close_popup' };
  function dispatch(msg) {
    if (!msg || !msg.type) return;
    const cmd = CMD[msg.type];
    if (cmd && T && T.core) T.core.invoke(cmd).catch((e) => console.error('invoke', cmd, e));
  }

  // 실제 분석 흐름이 시작되면 dev 패널 숨김
  function hideDevPanel() {
    const p = document.getElementById('codepop-dev-panel');
    if (p) p.style.display = 'none';
  }

  // ── 2) Rust → JS : 이벤트 → 기존 렌더 함수 호출 ──
  if (T && T.event) {
    T.event.listen('render',  (e) => { hideDevPanel(); window.__codepopRender  && window.__codepopRender(e.payload || {}); });
    T.event.listen('loading', (e) => { hideDevPanel(); window.__codepopLoading && window.__codepopLoading((e.payload && e.payload.code) || '', e.payload && e.payload.lang); });
    T.event.listen('error',   (e) => { hideDevPanel(); window.__codepopError   && window.__codepopError(e.payload || {}); });
  }

  // ── ESC 닫기 (Lua escHotkey 대체) ──
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') dispatch({ type: 'close' });
  });

  // ── 개발 프리뷰: ?preview=1 → 백엔드 없이 샘플 렌더 (브라우저 단독 시각 확인용) ──
  if (new URLSearchParams(location.search).has('preview')) {
    const SAMPLE = {
      code: [
        'function debounce(fn, delay) {',
        '  let timer = null;',
        '  return function (...args) {',
        '    clearTimeout(timer);',
        '    timer = setTimeout(() => fn.apply(this, args), delay);',
        '  };',
        '}',
      ].join('\n'),
      explain: [
        '## 🎯 한 줄',
        '**연속 호출 중 마지막 한 번만 실행되도록 묶는 디바운스 유틸**',
        '',
        '## 📍 흐름',
        '- ① 타이머 보관용 클로저 생성 `L2`',
        '- ② 호출마다 이전 타이머 취소 후 재예약 ⭐ `L4-5`',
        '',
        '## 💡 핵심',
        '- `L2`: 클로저로 timer ID 캡처 — 호출 간 상태 유지',
        '- `L4`: 이전 예약 취소가 디바운스의 본질',
        '',
        '## ⚠️ 의심점',
        '- L5: this 바인딩을 apply 로 넘기지만 화살표라 상위 this 고정',
        '',
        '## 🔎 라인별',
        '`L2: 클로저로 timer ID 캡처`',
        '`L4: 이전 timer 취소`',
        '`L5: delay 후 fn 실행 재예약`',
      ].join('\n'),
      source: 'cache',
      elapsed: 0,
      lang: 'ko',
    };
    const go = () => window.__codepopRender && window.__codepopRender(SAMPLE);
    if (document.readyState === 'loading') window.addEventListener('DOMContentLoaded', go);
    else go();
    return;
  }

  // ── Phase 2 임시 dev 입력 패널 ──
  // 선택 캡처(Phase 3) 전까지, tauri 안에서 코드를 직접 붙여넣어 전체 흐름을 테스트.
  // 설명이 비어있는 초기 상태에서만 노출. (Phase 4/5 에서 제거 예정)
  if (T && T.core) {
    window.addEventListener('DOMContentLoaded', () => {
      const explain = document.getElementById('explain');
      const panel = document.createElement('div');
      panel.id = 'codepop-dev-panel';
      panel.style.cssText =
        'position:fixed;left:12px;right:12px;bottom:12px;z-index:9999;' +
        'background:#232325;border:1px solid #3a3a3d;border-radius:10px;padding:10px;' +
        'display:flex;flex-direction:column;gap:8px;box-shadow:0 8px 24px rgba(0,0,0,.4);';
      panel.innerHTML =
        '<div style="font-size:11px;color:#7a7a7d;">🧪 dev — 분석할 코드 붙여넣기 (Phase 3 에서 단축키로 대체)</div>' +
        '<textarea id="dev-input" rows="4" style="width:100%;resize:vertical;background:#131315;color:#c5c5c8;' +
        'border:1px solid #2c2c2e;border-radius:6px;padding:8px;font-family:SF Mono,monospace;font-size:11.5px;"></textarea>' +
        '<button id="dev-go" style="align-self:flex-end;background:#ff8c42;color:#1e1e20;border:0;border-radius:6px;' +
        'padding:6px 14px;font-weight:700;font-size:12px;cursor:pointer;">✦ 분석</button>';
      document.body.appendChild(panel);
      const hide = () => { panel.style.display = 'none'; };
      document.getElementById('dev-go').addEventListener('click', () => {
        const text = document.getElementById('dev-input').value;
        if (!text || text.trim().length < 5) return;
        hide();
        T.core.invoke('dev_analyze', { text }).catch((e) => console.error('dev_analyze', e));
      });
    });
  }
})();
