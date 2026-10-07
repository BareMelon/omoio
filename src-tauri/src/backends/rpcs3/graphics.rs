//! The picture, sized for the machine it runs on.
//!
//! RPCS3 draws every game at the PS3's own 1280x720 and multiplies that by
//! `Resolution Scale`, a percentage that ships at 100. On a 1440p screen that
//! means a 720p picture stretched to twice its size, which is the blur most
//! people blame on emulation. Omoio sets the scale once, from the display and
//! the graphics card, and never again: a value the user picks is theirs.
//!
//! Two numbers go into it, and only one is arithmetic:
//!
//! - **The display** decides what fills the screen: its height over 720. That
//!   part is exact.
//! - **Graphics memory** decides what the card can carry. Drawing at twice the
//!   resolution needs four times the pixels, and RPCS3 keeps several buffers of
//!   that size. The limits below are our judgement, not a published table;
//!   they are cautious on purpose, since a picture that is slightly soft is
//!   better than a game that stutters.

use tauri::AppHandle;

const KEY: &str = "Resolution Scale";
/// What RPCS3 ships with. Anything else was chosen by someone and is left.
const SHIPPED: u32 = 100;

/// The scale that fills this display without asking more of the card than it
/// has, in RPCS3's steps of 50.
pub fn recommended_scale(display_height: u32, graphics_memory: u64) -> u32 {
    const GIB: u64 = 1024 * 1024 * 1024;

    // What fills the screen, to the nearest step: 1080p is 150, 1440p is 200,
    // 2160p is 300.
    let fill = ((display_height as f64 / 720.0 * 2.0).round() as u32 * 50).clamp(100, 300);

    let most = match graphics_memory {
        m if m < 3 * GIB => 100,
        m if m < 6 * GIB => 150,
        m if m < 10 * GIB => 200,
        _ => 300,
    };

    fill.min(most)
}

/// The scale currently in RPCS3's settings, if it has written them yet.
fn current(config: &str) -> Option<u32> {
    config
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with(KEY) && line[KEY.len()..].starts_with(':'))
        .and_then(|line| line[KEY.len() + 1..].trim().parse().ok())
}

/// Sets the scale for this machine if it is still at RPCS3's own default.
/// Returns the scale this machine should run at, or `None` when the user had
/// already chosen one of their own.
///
/// An error means RPCS3 has not written its settings yet, which it does on
/// first launch. The caller tries again next time rather than marking this as
/// done.
pub fn apply(app: &AppHandle, display_height: u32, graphics_memory: u64) -> Result<Option<u32>, String> {
    let path = super::config_dir(app)?.join("config.yml");
    let config = std::fs::read_to_string(&path)
        .map_err(|_| "RPCS3 hasn't written its settings yet.".to_string())?;
    if current(&config) != Some(SHIPPED) {
        return Ok(None);
    }

    let scale = recommended_scale(display_height, graphics_memory);
    if scale == SHIPPED {
        // Already right for this machine; nothing to write.
        return Ok(Some(scale));
    }
    let updated = super::account::replace_setting(&config, KEY, &scale.to_string());
    std::fs::write(&path, updated).map_err(|e| e.to_string())?;
    Ok(Some(scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn the_display_decides_what_fills_it() {
        assert_eq!(recommended_scale(720, 16 * GIB), 100);
        assert_eq!(recommended_scale(1080, 16 * GIB), 150);
        assert_eq!(recommended_scale(1440, 16 * GIB), 200);
        assert_eq!(recommended_scale(2160, 16 * GIB), 300);
    }

    #[test]
    fn a_small_card_holds_it_back() {
        assert_eq!(recommended_scale(2160, 2 * GIB), 100);
        assert_eq!(recommended_scale(2160, 4 * GIB), 150);
        assert_eq!(recommended_scale(2160, 8 * GIB), 200);
        assert_eq!(recommended_scale(1440, 4 * GIB), 150);
    }

    #[test]
    fn an_odd_display_rounds_to_a_step_and_stays_in_range() {
        assert_eq!(recommended_scale(1200, 16 * GIB), 150);
        assert_eq!(recommended_scale(1600, 16 * GIB), 200);
        assert_eq!(recommended_scale(480, 16 * GIB), 100);
        assert_eq!(recommended_scale(4320, 16 * GIB), 300);
    }

    #[test]
    fn only_the_scale_line_is_read() {
        // "Resolution" sits right above it and starts the same way.
        let config = "Video:\n  Resolution: 1280x720\n  Resolution Scale: 100\n";
        assert_eq!(current(config), Some(100));
        assert_eq!(current("Video:\n  Resolution: 1280x720\n"), None);
    }
}
