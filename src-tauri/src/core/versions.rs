//! Comparing emulator releases.
//!
//! RPCS3 names a build like `0.0.42-19985-6ba56a52`: the build number is what
//! orders them, and the commit hash after it says nothing about order. Cemu
//! tags a release like `v2.6`. Both are read as numbers piece by piece, up to
//! the first piece that is not a number.

fn numbers(version: &str) -> Vec<u64> {
    let version = version.trim();
    version
        .strip_prefix('v')
        .unwrap_or(version)
        // A numeric-only RPCS3 commit hash is still a hash, not a version
        // component. Mac bundle metadata contains only version and build.
        .split('-')
        .take(2)
        .flat_map(|part| part.split('.'))
        .map_while(|piece| piece.parse().ok())
        .collect()
}

/// Whether `newest` is a later release than `installed`.
pub fn is_newer_release(newest: &str, installed: &str) -> bool {
    let (a, b) = (numbers(newest), numbers(installed));
    if a.is_empty() {
        return false;
    }
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpcs3_builds_go_by_their_build_number() {
        assert!(is_newer_release("0.0.42-19985-6ba56a52", "0.0.42-19981-954d7968"));
        assert!(!is_newer_release("0.0.42-19981-954d7968", "0.0.42-19985-6ba56a52"));
        assert!(!is_newer_release("0.0.42-19985-6ba56a52", "0.0.42-19985-6ba56a52"));
        assert!(is_newer_release("0.0.43-20001-0a1b2c3d", "0.0.42-19985-6ba56a52"));
    }

    #[test]
    fn mac_bundle_build_numbers_compare_without_a_commit_hash() {
        assert!(!is_newer_release("0.0.43-20240-12345678", "0.0.43-20240"));
        assert!(!is_newer_release("0.0.43-20240-5f8dd1de", "0.0.43-20240"));
        assert!(is_newer_release("0.0.43-20241-12345678", "0.0.43-20240"));
        assert!(!is_newer_release("0.0.43-20239-12345678", "0.0.43-20240"));
    }

    #[test]
    fn cemu_tags_compare_as_numbers() {
        assert!(is_newer_release("2.7", "2.6"));
        assert!(is_newer_release("2.10", "2.9"), "not as decimals");
        assert!(!is_newer_release("2.6", "2.6"));
        assert!(!is_newer_release("v2.6", "2.6"));
    }

    #[test]
    fn nothing_readable_is_never_newer() {
        assert!(!is_newer_release("", "2.6"));
        assert!(!is_newer_release("nightly", "2.6"));
    }
}
