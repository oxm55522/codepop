// selection.rs — 선택 텍스트 캡처 (macOS 네이티브). Lua getSelectedText 포팅.
// 흐름: 클립보드 전체 백업 → clear → ⌘C 합성 → changeCount 폴링 → 읽기 → 복원.
// changeCount 폴링은 고정 sleep 대신 사용 (Slack/Notion 등 ⌘C 응답 느린 앱 대응).

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2::rc::Retained;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString};
use objc2_foundation::{NSArray, NSData, NSString};
use std::thread::sleep;
use std::time::{Duration, Instant};

const POLL_MS: u64 = 500; // CLIPBOARD_POLL_MS
const STEP_MS: u64 = 20; // CLIPBOARD_STEP_US(20000)
const KEYCODE_C: core_graphics::event::CGKeyCode = 8; // 'c'

/// 한 NSPasteboardItem 의 (타입, 데이터) 쌍들
struct SavedItem {
    entries: Vec<(Retained<NSString>, Retained<NSData>)>,
}

fn general() -> Retained<NSPasteboard> {
    NSPasteboard::generalPasteboard()
}

/// 클립보드 모든 아이템/타입 백업 (이미지·파일 포함 — 텍스트만 백업하면 손실)
fn backup(pb: &NSPasteboard) -> Vec<SavedItem> {
    let mut out = Vec::new();
    let Some(items) = pb.pasteboardItems() else { return out };
    for i in 0..items.count() {
        let item = items.objectAtIndex(i);
        let mut entries = Vec::new();
        let types = item.types();
        for j in 0..types.count() {
            let t = types.objectAtIndex(j);
            if let Some(data) = item.dataForType(&t) {
                entries.push((t, data));
            }
        }
        if !entries.is_empty() {
            out.push(SavedItem { entries });
        }
    }
    out
}

/// 백업본으로 클립보드 원복
fn restore(pb: &NSPasteboard, saved: &[SavedItem]) {
    if saved.is_empty() {
        return;
    }
    pb.clearContents();
    let mut protos: Vec<Retained<objc2::runtime::ProtocolObject<dyn objc2_app_kit::NSPasteboardWriting>>> =
        Vec::new();
    for s in saved {
        let item = NSPasteboardItem::new();
        for (t, data) in &s.entries {
            item.setData_forType(data, t);
        }
        protos.push(objc2::runtime::ProtocolObject::from_retained(item));
    }
    let arr = NSArray::from_retained_slice(&protos);
    pb.writeObjects(&arr);
}

/// CGEvent 로 ⌘C 합성 (Accessibility 권한 필요)
fn send_cmd_c() {
    let Ok(src) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) else { return };
    if let Ok(down) = CGEvent::new_keyboard_event(src.clone(), KEYCODE_C, true) {
        down.set_flags(CGEventFlags::CGEventFlagCommand);
        down.post(CGEventTapLocation::HID);
    }
    if let Ok(up) = CGEvent::new_keyboard_event(src, KEYCODE_C, false) {
        up.set_flags(CGEventFlags::CGEventFlagCommand);
        up.post(CGEventTapLocation::HID);
    }
}

/// 현재 선택된 텍스트 캡처. 선택 없음/실패 시 None.
pub fn get_selected_text() -> Option<String> {
    let pb = general();
    let saved = backup(&pb);

    let before = pb.changeCount();
    pb.clearContents();
    send_cmd_c();

    // changeCount > before+1 (clear=+1, copy=+1) 이면 실제 복사 발생
    let deadline = Instant::now() + Duration::from_millis(POLL_MS);
    loop {
        if pb.changeCount() > before + 1 {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        sleep(Duration::from_millis(STEP_MS));
    }

    let after = pb.changeCount();
    let string_type = unsafe { NSPasteboardTypeString };
    let selected = pb.stringForType(string_type).map(|s| s.to_string());

    restore(&pb, &saved);

    if after <= before + 1 {
        return None; // 복사가 일어나지 않음 = 선택 없음
    }
    match selected {
        Some(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// Accessibility(손쉬운 사용) 권한 여부. prompt=true 면 시스템 권한 요청 다이얼로그 표시.
pub fn accessibility_trusted(prompt: bool) -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::CFString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    }

    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let val = CFBoolean::from(prompt);
    let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), val.as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
}
