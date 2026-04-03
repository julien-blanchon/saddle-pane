//! Shared cache directory resolution for SVG responses and collection icon lists.

use std::path::PathBuf;

/// Root cache directory for bevy_iconify.
///
/// Resolution order:
/// 1. `BEVY_ICONIFY_CACHE_DIR` env var
/// 2. Platform-specific:
///    - macOS: `~/Library/Caches/bevy_iconify/`
///    - Linux: `/tmp/bevy_iconify/`
///    - Windows: `%LOCALAPPDATA%/cache/bevy_iconify/`
pub fn root() -> PathBuf {
    if let Ok(dir) = std::env::var("BEVY_ICONIFY_CACHE_DIR") {
        return PathBuf::from(dir);
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join("Library/Caches/bevy_iconify"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/bevy_iconify"))
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/tmp/bevy_iconify")
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join("cache/bevy_iconify"))
            .unwrap_or_else(|_| PathBuf::from("bevy_iconify"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/tmp/bevy_iconify")
    }
}

/// Directory for cached SVG responses.
#[cfg(all(not(test), feature = "cache"))]
pub fn svg_dir() -> PathBuf {
    root().join("svg")
}

/// Directory for cached collection icon lists (used for fuzzy suggestions).
pub fn collections_dir() -> PathBuf {
    root().join("collections")
}
