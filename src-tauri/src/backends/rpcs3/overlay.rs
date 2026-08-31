//! Putting the game picture inside Omoio's window.
//!
//! RPCS3 stays its own process, as it must. What changes is where its window
//! sits: stripped of its frame, owned by Omoio, and parked exactly over the
//! content area, so playing a game looks like part of the app rather than a
//! second program appearing.
//!
//! Reparenting the window into ours was tried first and does not work: Tauri
//! draws through WebView2, which composites over native child windows whatever
//! their z-order, so the game ran but stayed invisible. Owning the window
//! instead keeps it above the webview and out of the taskbar.

use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindowVisible, SetWindowLongPtrW,
    SetWindowPos, GWLP_HWNDPARENT, GWL_STYLE, SWP_NOACTIVATE, SWP_NOZORDER, WS_CAPTION, WS_POPUP,
    WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

struct Search {
    pid: u32,
    found: Option<HWND>,
}

unsafe extern "system" fn visit(window: HWND, state: LPARAM) -> BOOL {
    let search = unsafe { &mut *(state.0 as *mut Search) };
    let mut owner = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut owner)) };

    if owner == search.pid && unsafe { IsWindowVisible(window) }.as_bool() {
        search.found = Some(window);
        return BOOL(0); // stop at the first one
    }
    BOOL(1)
}

/// RPCS3's visible window for a given process. Called on a timer while the
/// game boots, because the window only appears once it has something to show.
pub fn find_window(pid: u32) -> Option<isize> {
    let mut search = Search { pid, found: None };
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    search.found.map(|hwnd| hwnd.0 as isize)
}

/// Takes the frame off the game window and makes Omoio its owner, so it rides
/// above Omoio, minimises with it, and never gets its own taskbar button.
pub fn attach(game: isize, host: isize) {
    let game = HWND(game as *mut _);
    unsafe {
        let style = GetWindowLongPtrW(game, GWL_STYLE);
        let stripped = style
            & !((WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0) as isize)
            | ((WS_POPUP.0 | WS_VISIBLE.0) as isize);
        SetWindowLongPtrW(game, GWL_STYLE, stripped);
        SetWindowLongPtrW(game, GWLP_HWNDPARENT, host);
    }
}

pub fn place(game: isize, x: i32, y: i32, width: i32, height: i32) {
    if width <= 0 || height <= 0 {
        return;
    }
    unsafe {
        // Not activated and not reordered: moving the picture should never
        // steal focus from whatever the user is doing.
        let _ = SetWindowPos(
            HWND(game as *mut _),
            None,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}
