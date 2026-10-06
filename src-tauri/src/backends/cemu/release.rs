//! The one Cemu release Omoio installs: the newest it has been checked
//! against.
//!
//! Omoio's Cemu code was read out of Cemu's source at one tag: the folders it
//! uses, the settings it writes, and above all how its SDL input names a pad
//! (sdl.rs), which a profile has to match byte for byte. A later Cemu can
//! change any of that. Cemu's own main branch has moved on to SDL 3 since
//! v2.6 (its `vcpkg.json`), and SDL 3 names a Bluetooth pad differently and
//! reports a Switch pad's face buttons by place (`HIDAPI_AddDevice`, its
//! migration notes), so a profile written for v2.6 would leave such a pad
//! without a player. Omoio updates Cemu without
//! asking, so it goes no further than this release until the next one has
//! been read and tried.
//!
//! The download is a fixed file, so it is checked against the SHA-256 it had
//! when it was checked (6 October 2026). Cemu publishes no checksum of its own.

/// The release, by its tag.
pub const TAG: &str = "v2.6";

/// The Windows build of `TAG`, from Cemu's own GitHub releases.
pub const DOWNLOAD: &str = "https://github.com/cemu-project/Cemu/releases/download/v2.6/cemu-2.6-windows-x64.zip";

/// The file's name, which is what it is saved as while it downloads.
pub const FILE: &str = "cemu-2.6-windows-x64.zip";

/// SHA-256 of `DOWNLOAD`, as hex.
pub const SHA256: &str = "a6bcc2bc42a362d10213819948f3152fae7d47f70067f25939b51d3ddcfb0896";

/// `TAG` as `detect_version` reports a version, without the `v`.
pub fn version() -> &'static str {
    TAG.strip_prefix('v').unwrap_or(TAG)
}

/// Whether the Cemu installed, by the version `detect_version` reports, is
/// the release Omoio was checked against, so its SDL pads are named the way
/// sdl.rs names them. Any other version, or one not known, is not: its
/// PlayStation and Switch pads are then left to stand in for or warn about,
/// rather than named in a way Cemu may not find.
pub fn names_sdl_pads_as_checked(installed: Option<&str>) -> bool {
    installed.is_some_and(|got| got.trim().strip_prefix('v').unwrap_or(got.trim()) == version())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::versions::is_newer_release;

    #[test]
    fn the_download_is_the_tagged_release() {
        assert_eq!(version(), "2.6");
        assert!(DOWNLOAD.contains(&format!("/releases/download/{TAG}/")));
        assert!(DOWNLOAD.ends_with(&format!("/{FILE}")));
        assert!(FILE.ends_with("-windows-x64.zip"), "the Windows build");
        assert!(FILE.contains(version()));
        assert_eq!(SHA256.len(), 64);
        assert!(SHA256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    }

    #[test]
    fn an_older_cemu_is_updated_to_the_checked_release_and_no_further() {
        // What the update check does with `newest_version`, which is this
        // release.
        assert!(is_newer_release(version(), "2.5"));
        assert!(is_newer_release(version(), "2.0-94"));
        assert!(!is_newer_release(version(), "2.6"), "the checked release is kept");
        assert!(!is_newer_release(version(), "2.7"), "a newer Cemu is never taken back");
    }

    #[test]
    fn only_the_checked_release_has_its_sdl_pads_named() {
        assert!(names_sdl_pads_as_checked(Some("2.6")));
        assert!(names_sdl_pads_as_checked(Some("v2.6")));
        assert!(!names_sdl_pads_as_checked(Some("2.7")), "not read yet");
        assert!(!names_sdl_pads_as_checked(Some("2.5")), "not read either");
        assert!(!names_sdl_pads_as_checked(Some("2.6.1")));
        assert!(!names_sdl_pads_as_checked(None), "no version file");
    }
}
