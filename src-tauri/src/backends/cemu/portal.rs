//! Figures on Cemu's Skylanders portal, put there from outside Cemu.
//!
//! Cemu keeps its portal in memory and fills it only from its own Emulated
//! USB Devices window; there is no file or command-line option for it. Omoio
//! opens that window through its menu item, presses its buttons with window
//! messages, reads what each slot holds, and closes it again. Nothing moves
//! the mouse, and each window is put out of sight as it opens so it never
//! covers the game. How this was proven is in docs/what-we-verified.md,
//! "Skylanders".

use crate::core::figures::Character;
use std::path::Path;
use std::time::{Duration, Instant};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetDlgItem, GetMenu, GetMenuItemCount, GetMenuItemID,
    GetMenuStringW, GetParent, GetSubMenu, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, SendMessageTimeoutW,
    SetWindowPos, HMENU, MF_BYPOSITION, SMTO_ABORTIFHUNG, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, WM_CLOSE,
    WM_COMMAND, WM_GETTEXT, WM_SETTEXT,
};

/// How many figures Cemu's portal holds (`MAX_SKYLANDERS`).
pub const SLOTS: usize = 16;

/// The window, its menu item and its file window, as Cemu 2.6 titles them.
const WINDOW: &str = "Emulated USB Devices";
const OPEN_FIGURE: &str = "Open Skylander dump";

/// Cemu's figure maker and the save window it opens
/// (`EmulatedUSBDeviceFrame.cpp`, `CreateSkylanderDialog`).
const CREATOR: &str = "Skylander Figure Creator";
const SAVE_FIGURE: &str = "Create Skylander file";

/// Asking a list box how many items it has, one item's text and its length,
/// and the number kept with it. Windows carries these across processes.
const CB_GETCOUNT: u32 = 0x0146;
const CB_GETLBTEXT: u32 = 0x0148;
const CB_GETLBTEXTLEN: u32 = 0x0149;
const CB_GETITEMDATA: u32 = 0x0150;

const LOOKS_DIFFERENT: &str = "Cemu's portal looks different from what Omoio knows.";

/// A button press, sent as the button itself would get it.
const BM_CLICK: u32 = 0x00F5;
/// The file name box and the Open button in a Windows file window.
const FILE_NAME_BOX: i32 = 0x047C;
const OPEN_BUTTON: i32 = 1;

const WAIT: Duration = Duration::from_secs(5);

fn text(window: HWND) -> String {
    let mut buffer = [0u16; 512];
    let mut copied = 0usize;
    // Windows carries the text across to Cemu's process and back. A window
    // that has stopped answering is given up on rather than waited for.
    unsafe {
        SendMessageTimeoutW(
            window,
            WM_GETTEXT,
            WPARAM(buffer.len()),
            LPARAM(buffer.as_mut_ptr() as isize),
            SMTO_ABORTIFHUNG,
            2000,
            Some(&mut copied),
        )
    };
    String::from_utf16_lossy(&buffer[..copied.min(buffer.len())])
}

fn set_text(window: HWND, value: &str) {
    let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        SendMessageTimeoutW(
            window,
            WM_SETTEXT,
            WPARAM(0),
            LPARAM(wide.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            2000,
            None,
        )
    };
}

fn class(window: HWND) -> String {
    let mut buffer = [0u16; 128];
    let length = unsafe { GetClassNameW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn press(window: HWND) {
    // Posted rather than sent: the press can open a window that waits for an
    // answer, and waiting on it here would wait forever.
    let _ = unsafe { PostMessageW(Some(window), BM_CLICK, WPARAM(0), LPARAM(0)) };
}

fn close(window: HWND) {
    let _ = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
}

/// Far off the screen, where it still works but nobody sees it.
fn out_of_sight(window: HWND) {
    let _ = unsafe {
        SetWindowPos(
            window,
            None,
            -32000,
            -32000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
}

unsafe extern "system" fn collect(window: HWND, list: LPARAM) -> BOOL {
    let list = unsafe { &mut *(list.0 as *mut Vec<HWND>) };
    list.push(window);
    BOOL(1)
}

fn children(window: HWND) -> Vec<HWND> {
    let mut list: Vec<HWND> = Vec::new();
    let _ = unsafe { EnumChildWindows(Some(window), Some(collect), LPARAM(&mut list as *mut Vec<HWND> as isize)) };
    list
}

/// Cemu's windows that are showing, in the order Windows keeps them.
fn windows_of(pid: u32) -> Vec<HWND> {
    let mut all: Vec<HWND> = Vec::new();
    let _ = unsafe { EnumWindows(Some(collect), LPARAM(&mut all as *mut Vec<HWND> as isize)) };
    all.into_iter()
        .filter(|&window| {
            let mut owner = 0u32;
            unsafe { GetWindowThreadProcessId(window, Some(&mut owner)) };
            owner == pid && unsafe { IsWindowVisible(window) }.as_bool()
        })
        .collect()
}

fn wait_for(pid: u32, found: impl Fn(HWND) -> bool) -> Option<HWND> {
    let until = Instant::now() + WAIT;
    while Instant::now() < until {
        if let Some(window) = windows_of(pid).into_iter().find(|&w| found(w)) {
            return Some(window);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    None
}

/// The command a menu item sends, found by its text so it does not matter
/// what number wxWidgets gave it this time.
fn menu_command(menu: HMENU, wanted: &str) -> Option<u32> {
    let count = unsafe { GetMenuItemCount(Some(menu)) };
    for at in 0..count.max(0) {
        let sub = unsafe { GetSubMenu(menu, at) };
        if !sub.is_invalid() {
            if let Some(found) = menu_command(sub, wanted) {
                return Some(found);
            }
            continue;
        }
        let mut buffer = [0u16; 128];
        let length = unsafe { GetMenuStringW(menu, at as u32, Some(&mut buffer), MF_BYPOSITION) };
        let label = String::from_utf16_lossy(&buffer[..length.max(0) as usize]).replace('&', "");
        if label.starts_with(wanted) {
            return Some(unsafe { GetMenuItemID(menu, at) });
        }
    }
    None
}

/// The Emulated USB Devices window, opened if it is not already, and put out
/// of sight.
fn open(pid: u32) -> Result<HWND, String> {
    if let Some(window) = windows_of(pid).into_iter().find(|&w| text(w) == WINDOW) {
        out_of_sight(window);
        return Ok(window);
    }
    let main = windows_of(pid)
        .into_iter()
        .find(|&w| !unsafe { GetMenu(w) }.is_invalid())
        .ok_or("Cemu isn't answering. Try again once the game has started.")?;
    let command = menu_command(unsafe { GetMenu(main) }, WINDOW)
        .ok_or("This Cemu has no Emulated USB Devices window.")?;
    let _ = unsafe { PostMessageW(Some(main), WM_COMMAND, WPARAM(command as usize), LPARAM(0)) };
    let window = wait_for(pid, |w| text(w) == WINDOW).ok_or("Cemu's portal didn't open. Try again.")?;
    out_of_sight(window);
    Ok(window)
}

/// Controls of one class and text, in the order Cemu made them. The
/// Skylanders page is made first, so its sixteen rows come before the other
/// toys' pages.
fn controls(window: HWND, class_has: &str, label: Option<&str>) -> Vec<HWND> {
    children(window)
        .into_iter()
        .filter(|&c| class(c).contains(class_has) && label.map_or(true, |l| text(c) == l))
        .collect()
}

/// What each slot holds, empty where it holds nothing.
fn read(window: HWND) -> Vec<String> {
    controls(window, "Edit", None)
        .into_iter()
        .take(SLOTS)
        .map(|slot| {
            let name = text(slot);
            if name == "None" {
                String::new()
            } else {
                name
            }
        })
        .collect()
}

fn check(slot: usize) -> Result<(), String> {
    if slot < SLOTS {
        Ok(())
    } else {
        Err("The portal has no slot there.".to_string())
    }
}

/// Clicks OK on a message Cemu put up, so it does not sit over the game, and
/// hands on what it said. Windows Omoio opened on purpose are left alone.
fn dismiss_message(pid: u32, expected: &[&str]) -> Option<String> {
    let message = windows_of(pid)
        .into_iter()
        .find(|&w| class(w) == "#32770" && !expected.contains(&text(w).as_str()))?;
    let said = children(message)
        .into_iter()
        .filter(|&c| class(c) == "Static")
        .map(text)
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if let Ok(ok) = unsafe { GetDlgItem(Some(message), OPEN_BUTTON) } {
        press(ok);
    }
    Some(said)
}

/// The figures on the portal, by slot.
pub fn figures(pid: u32) -> Result<Vec<String>, String> {
    let window = open(pid)?;
    let names = read(window);
    close(window);
    Ok(names)
}

/// Puts the figure in `file` on the portal in `slot`, counted from 0, and
/// returns what the portal holds afterwards.
pub fn load(pid: u32, slot: usize, file: &Path) -> Result<Vec<String>, String> {
    check(slot)?;
    let window = open(pid)?;
    let load = controls(window, "Button", Some("Load"))
        .into_iter()
        .nth(slot)
        .ok_or("Cemu's portal looks different from what Omoio knows.")?;
    press(load);

    let picker = wait_for(pid, |w| class(w) == "#32770" && text(w) == OPEN_FIGURE)
        .ok_or("Cemu didn't ask for the figure. Try again.")?;
    finish_file_window(pid, picker, file)?;
    if let Some(said) = dismiss_message(pid, &[OPEN_FIGURE]) {
        close(window);
        return Err(if said.is_empty() {
            "Cemu couldn't put that figure on the portal.".to_string()
        } else {
            format!("Cemu couldn't put that figure on the portal: {said}")
        });
    }
    let names = read(window);
    close(window);
    Ok(names)
}

/// Fills in a Windows file window Cemu opened, out of sight, presses its
/// Open or Save button, and waits for it to close.
fn finish_file_window(pid: u32, picker: HWND, file: &Path) -> Result<(), String> {
    const DIFFERENT: &str = "Cemu's file window looks different from what Omoio knows.";
    out_of_sight(picker);
    let name_box = unsafe { GetDlgItem(Some(picker), FILE_NAME_BOX) }
        .ok()
        .and_then(|combo| children(combo).into_iter().find(|&c| class(c) == "Edit"))
        .or_else(|| children(picker).into_iter().find(|&c| class(c) == "Edit"))
        .ok_or(DIFFERENT)?;
    set_text(name_box, &file.to_string_lossy());
    let button = unsafe { GetDlgItem(Some(picker), OPEN_BUTTON) }.map_err(|_| DIFFERENT.to_string())?;
    press(button);
    let until = Instant::now() + WAIT;
    while Instant::now() < until && windows_of(pid).contains(&picker) {
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn send(window: HWND, message: u32, wparam: usize, lparam: isize) -> usize {
    let mut result = 0usize;
    unsafe {
        SendMessageTimeoutW(
            window,
            message,
            WPARAM(wparam),
            LPARAM(lparam),
            SMTO_ABORTIFHUNG,
            2000,
            Some(&mut result),
        )
    };
    result
}

/// Cemu's figure maker, opened from the Create button of `slot` and put
/// out of sight.
fn open_creator(pid: u32, window: HWND, slot: usize) -> Result<HWND, String> {
    let create = controls(window, "Button", Some("Create"))
        .into_iter()
        .nth(slot)
        .ok_or(LOOKS_DIFFERENT)?;
    press(create);
    let creator = wait_for(pid, |w| text(w) == CREATOR).ok_or("Cemu's figure maker didn't open. Try again.")?;
    out_of_sight(creator);
    Ok(creator)
}

fn cancel_creator(creator: HWND) {
    if let Some(cancel) = controls(creator, "Button", Some("Cancel")).into_iter().next() {
        press(cancel);
    }
}

/// Every character Cemu's figure maker offers, read from its list: the name
/// each item shows and the id and variant it carries. The maker is closed
/// again without making anything.
pub fn characters(pid: u32) -> Result<Vec<Character>, String> {
    let window = open(pid)?;
    let creator = open_creator(pid, window, 0)?;
    let found: Vec<Character> = children(creator)
        .into_iter()
        .find(|&c| class(c) == "ComboBox")
        .map(|list| {
            let count = send(list, CB_GETCOUNT, 0, 0).min(4096);
            (0..count)
                .filter_map(|at| {
                    // A failed ask comes back as -1, so anything outlandish is skipped.
                    let length = send(list, CB_GETLBTEXTLEN, at, 0);
                    if length == 0 || length > 256 {
                        return None;
                    }
                    let mut buffer = vec![0u16; length + 1];
                    send(list, CB_GETLBTEXT, at, buffer.as_mut_ptr() as isize);
                    let name = String::from_utf16_lossy(&buffer[..length]);
                    Character::from_item(&name, send(list, CB_GETITEMDATA, at, 0) as u64)
                })
                .collect()
        })
        .unwrap_or_default();
    cancel_creator(creator);
    std::thread::sleep(Duration::from_millis(300));
    close(window);
    if found.is_empty() {
        Err("Couldn't read Cemu's list of characters.".to_string())
    } else {
        Ok(found)
    }
}

/// Has Cemu's figure maker make a figure of `character` into `file`. Cemu
/// then puts it on the portal in `slot` itself. Returns what the portal
/// holds afterwards.
pub fn create(pid: u32, slot: usize, character: &Character, file: &Path) -> Result<Vec<String>, String> {
    check(slot)?;
    let window = open(pid)?;
    let creator = open_creator(pid, window, slot)?;
    // The id and variant boxes, not the typing box inside the list above them.
    let boxes: Vec<HWND> = children(creator)
        .into_iter()
        .filter(|&c| class(c) == "Edit" && unsafe { GetParent(c) }.ok() == Some(creator))
        .collect();
    let (Some(&id_box), Some(&variant_box), 2) = (boxes.first(), boxes.get(1), boxes.len()) else {
        cancel_creator(creator);
        close(window);
        return Err(LOOKS_DIFFERENT.to_string());
    };
    set_text(id_box, &character.id.to_string());
    set_text(variant_box, &character.variant.to_string());
    let Some(make) = controls(creator, "Button", Some("Create")).into_iter().next() else {
        cancel_creator(creator);
        close(window);
        return Err(LOOKS_DIFFERENT.to_string());
    };
    press(make);

    let saver = wait_for(pid, |w| class(w) == "#32770" && text(w) == SAVE_FIGURE)
        .ok_or("Cemu didn't ask where to keep the figure. Try again.")?;
    finish_file_window(pid, saver, file)?;
    let until = Instant::now() + WAIT;
    while Instant::now() < until && windows_of(pid).contains(&creator) {
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(300));
    if let Some(said) = dismiss_message(pid, &[OPEN_FIGURE, SAVE_FIGURE, CREATOR]) {
        close(window);
        return Err(if said.is_empty() {
            "Cemu couldn't make that figure.".to_string()
        } else {
            format!("Cemu couldn't make that figure: {said}")
        });
    }
    let names = read(window);
    close(window);
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a Cemu running and a figure file, so it runs only by hand:
    /// `OMOIO_CEMU_PID=<pid> OMOIO_FIGURE=<file> cargo test portal_on_a_running_cemu -- --ignored`
    #[test]
    #[ignore]
    fn portal_on_a_running_cemu() {
        let pid: u32 = std::env::var("OMOIO_CEMU_PID").unwrap().parse().unwrap();
        let figure = std::env::var("OMOIO_FIGURE").unwrap();
        let before = figures(pid).unwrap();
        assert_eq!(before.len(), SLOTS);
        let loaded = load(pid, 0, Path::new(&figure)).unwrap();
        assert!(!loaded[0].is_empty(), "slot 1 holds the figure: {loaded:?}");
        let cleared = clear(pid, 0).unwrap();
        assert!(cleared[0].is_empty(), "slot 1 is empty again: {cleared:?}");
    }
}

/// Takes the figure in `slot` off the portal, and returns what is left.
pub fn clear(pid: u32, slot: usize) -> Result<Vec<String>, String> {
    check(slot)?;
    let window = open(pid)?;
    let button = controls(window, "Button", Some("Clear"))
        .into_iter()
        .nth(slot)
        .ok_or("Cemu's portal looks different from what Omoio knows.")?;
    press(button);
    std::thread::sleep(Duration::from_millis(300));
    let names = read(window);
    close(window);
    Ok(names)
}
