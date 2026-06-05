// popup.rs — 커서 옆 팝업 배치/표시. Lua popupFrame 클램프 로직 포팅.

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize};

/// 마우스 커서 근처에 팝업 윈도우를 배치하고 표시. 화면 경계로 클램프.
pub fn show_at_cursor(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };

    let size = win
        .outer_size()
        .unwrap_or(PhysicalSize::new(520, 600));
    let scale = win.scale_factor().unwrap_or(1.0);
    let offset = 20.0 * scale;

    // 모니터 작업영역 (physical px)
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let (mx, my, mw, mh) = match monitor {
        Some(m) => {
            let p = m.position();
            let s = m.size();
            (p.x as f64, p.y as f64, s.width as f64, s.height as f64)
        }
        None => (0.0, 0.0, 1440.0, 900.0),
    };

    let (w, h) = (size.width as f64, size.height as f64);

    // 커서 위치 (physical, top-left 원점). 실패 시 화면 중앙 폴백.
    let (cx, cy) = match app.cursor_position() {
        Ok(p) => (p.x, p.y),
        Err(_) => (mx + mw / 2.0, my + mh / 2.0),
    };

    let mut x = cx + offset;
    let mut y = cy + offset;
    if x + w > mx + mw {
        x = cx - w - offset;
    }
    if y + h > my + mh {
        y = my + mh - h - offset;
    }
    if x < mx {
        x = mx + offset;
    }
    if y < my {
        y = my + offset;
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
    let _ = win.show();
    let _ = win.set_focus();
}
