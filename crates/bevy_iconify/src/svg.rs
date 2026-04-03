use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{LitStr, Meta, Token};
use url::Url;

use crate::attrs::{get_lit_bool, get_lit_str};
use crate::suggest;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) enum IconifyFlip {
    Horizontal,
    Vertical,
    Both,
}

impl IconifyFlip {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Both => "horizontal,vertical",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum IconifyRotation {
    Deg90,
    Deg180,
    Deg270,
}

impl IconifyRotation {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Deg90 => "90deg",
            Self::Deg180 => "180deg",
            Self::Deg270 => "270deg",
        }
    }
}

// ---------------------------------------------------------------------------
// Input parsing
// ---------------------------------------------------------------------------

/// Parsed representation of a `svg!("pack:name", ...)` macro invocation.
#[derive(Debug)]
pub(crate) struct IconifyInput {
    pub pack: String,
    pub name: String,
    pub color: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub flip: Option<IconifyFlip>,
    pub rotate: Option<IconifyRotation>,
    pub view_box: Option<bool>,
}

impl IconifyInput {
    /// Base API URL, overridable via `BEVY_ICONIFY_URL`.
    fn api_url() -> String {
        std::env::var("BEVY_ICONIFY_URL")
            .unwrap_or_else(|_| "https://api.iconify.design".to_string())
    }

    /// Build the full Iconify SVG endpoint URL with query parameters.
    fn icon_url(&self) -> syn::Result<String> {
        let base = Self::api_url();
        let mut url = Url::parse(&format!("{base}/{}/{}.svg", self.pack, self.name))
            .map_err(|e| syn::Error::new(Span::call_site(), format!("failed to parse url: {e}")))?;

        {
            let mut q = url.query_pairs_mut();
            if let Some(ref v) = self.color {
                q.append_pair("color", v);
            }
            if let Some(ref v) = self.width {
                q.append_pair("width", v);
            }
            if let Some(ref v) = self.height {
                q.append_pair("height", v);
            }
            if let Some(ref v) = self.flip {
                q.append_pair("flip", v.as_str());
            }
            if let Some(ref v) = self.rotate {
                q.append_pair("rotate", v.as_str());
            }
            if let Some(true) = self.view_box {
                q.append_pair("box", "true");
            }
        }

        let mut s = url.to_string();
        if s.ends_with('?') {
            s.pop();
        }
        Ok(s)
    }
}

impl Parse for IconifyInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // First token: "pack:name"
        let icon_str: LitStr = input.parse()?;
        let icon_val = icon_str.value();
        let parts: Vec<&str> = icon_val.split(':').collect();

        if parts.len() != 2 {
            return Err(syn::Error::new_spanned(
                &icon_str,
                "expected `pack_name:icon_name` (e.g. `\"mdi:home\"`)",
            ));
        }

        let pack = parts[0].to_string();
        let name = parts[1].to_string();

        if pack.is_empty() || name.is_empty() {
            return Err(syn::Error::new_spanned(
                &icon_str,
                "both pack name and icon name must be non-empty (e.g. `\"mdi:home\"`)",
            ));
        }

        let mut result = IconifyInput {
            pack,
            name,
            color: None,
            width: None,
            height: None,
            flip: None,
            rotate: None,
            view_box: None,
        };

        // Optional comma-separated key=value attributes
        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break; // trailing comma
            }

            let meta: Meta = input.parse()?;
            match meta {
                Meta::NameValue(nv) => {
                    let ident = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| {
                            syn::Error::new_spanned(&nv.path, "expected a simple identifier")
                        })?
                        .to_string();

                    match ident.as_str() {
                        "color" => {
                            result.color = Some(get_lit_str("color", &nv.value)?.value());
                        }
                        "width" => {
                            result.width = Some(get_lit_str("width", &nv.value)?.value());
                        }
                        "height" => {
                            result.height = Some(get_lit_str("height", &nv.value)?.value());
                        }
                        "flip" => {
                            let val = get_lit_str("flip", &nv.value)?.value();
                            result.flip = Some(match val.as_str() {
                                "horizontal" => IconifyFlip::Horizontal,
                                "vertical" => IconifyFlip::Vertical,
                                "both" | "horizontal,vertical" => IconifyFlip::Both,
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        &nv.value,
                                        "expected \"horizontal\", \"vertical\", or \"both\"",
                                    ));
                                }
                            });
                        }
                        "rotate" => {
                            let val = get_lit_str("rotate", &nv.value)?.value();
                            result.rotate = Some(match val.as_str() {
                                "90" | "90deg" => IconifyRotation::Deg90,
                                "180" | "180deg" => IconifyRotation::Deg180,
                                "270" | "270deg" => IconifyRotation::Deg270,
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        &nv.value,
                                        "expected \"90\", \"180\", or \"270\"",
                                    ));
                                }
                            });
                        }
                        "view_box" => {
                            result.view_box = Some(get_lit_bool("view_box", &nv.value)?);
                        }
                        other => {
                            return Err(syn::Error::new_spanned(
                                &nv.path,
                                format!(
                                    "unknown attribute `{other}`. \
                                     Expected: color, width, height, flip, rotate, view_box"
                                ),
                            ));
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &meta,
                        "expected `key = value` attribute",
                    ));
                }
            }
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

/// Compute a short blake3 hash of the URL for cache file naming.
/// Different parameters (color, size) produce different hashes.
#[cfg(any(all(not(test), feature = "cache"), feature = "offline"))]
fn hash_digest(url: &str) -> String {
    let mut buf = [0u8; 8];
    let mut hasher = blake3::Hasher::new();
    hasher.update(url.as_bytes());
    let mut reader = hasher.finalize_xof();
    reader.fill(&mut buf);
    hex::encode(buf)
}

/// Cache file path: `<cache_dir>/svg/<pack>/<name>-<hash>.svg`
#[cfg(all(not(test), feature = "cache"))]
fn svg_cache_path(pack: &str, name: &str, url: &str) -> std::path::PathBuf {
    crate::cache::svg_dir()
        .join(pack)
        .join(format!("{name}-{}.svg", hash_digest(url)))
}

// ---------------------------------------------------------------------------
// Offline mode
// ---------------------------------------------------------------------------

#[cfg(feature = "offline")]
fn offline_dir() -> std::path::PathBuf {
    std::env::var("BEVY_ICONIFY_OFFLINE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(
                std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()),
            )
            .join("icons")
        })
}

#[cfg(feature = "offline")]
fn offline_icon_path(pack: &str, name: &str, url: &str) -> std::path::PathBuf {
    offline_dir()
        .join(pack)
        .join(format!("{name}-{}.svg", hash_digest(url)))
}

#[cfg(feature = "offline")]
fn is_prepare_mode() -> bool {
    std::env::var("BEVY_ICONIFY_PREPARE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

#[cfg(feature = "offline")]
fn read_offline_svg(pack: &str, name: &str, url: &str) -> syn::Result<String> {
    let path = offline_icon_path(pack, name, url);
    std::fs::read_to_string(&path).map_err(|e| {
        syn::Error::new(
            Span::call_site(),
            format!(
                "failed to read offline icon at `{}`: {e}.\n\
                 Prepare icons first with: BEVY_ICONIFY_PREPARE=true cargo check",
                path.display()
            ),
        )
    })
}

#[cfg(feature = "offline")]
fn write_offline_svg(pack: &str, name: &str, url: &str, svg: &str) {
    let path = offline_icon_path(pack, name, url);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, svg);
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

/// Fetch an SVG from the Iconify API, with cache and error suggestions.
fn fetch_svg_from_api(input: &IconifyInput) -> syn::Result<String> {
    let url = input.icon_url()?;

    // Check cache (disabled during tests so they always hit the API)
    #[cfg(all(not(test), feature = "cache"))]
    {
        let cache_path = svg_cache_path(&input.pack, &input.name, &url);
        if let Ok(cached) = std::fs::read_to_string(&cache_path) {
            return Ok(cached);
        }
    }

    // Fetch from API — handle both HTTP 404 and legacy "200 with body 404"
    let text = match ureq::get(&url).call() {
        Ok(response) => response.into_string().map_err(|e| {
            syn::Error::new(
                Span::call_site(),
                format!("failed to read response body: {e}"),
            )
        })?,
        Err(ureq::Error::Status(404, _)) => {
            let api_url = IconifyInput::api_url();
            let msg = suggest::suggest_error_message(&input.pack, &input.name, &url, &api_url);
            return Err(syn::Error::new(Span::call_site(), msg));
        }
        Err(e) => {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("failed to fetch icon: {e}"),
            ));
        }
    };

    if text == "404" {
        let api_url = IconifyInput::api_url();
        let msg = suggest::suggest_error_message(&input.pack, &input.name, &url, &api_url);
        return Err(syn::Error::new(Span::call_site(), msg));
    }

    // Write to cache
    #[cfg(all(not(test), feature = "cache"))]
    {
        let cache_path = svg_cache_path(&input.pack, &input.name, &url);
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&cache_path, &text);
    }

    Ok(text)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Parse macro input, fetch SVG, return as a string literal token.
pub(crate) fn iconify_svg_impl(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<IconifyInput>(input)?;

    #[cfg(feature = "offline")]
    let svg = {
        if is_prepare_mode() {
            let svg = fetch_svg_from_api(&input)?;
            let url = input.icon_url()?;
            write_offline_svg(&input.pack, &input.name, &url, &svg);
            svg
        } else {
            let url = input.icon_url()?;
            read_offline_svg(&input.pack, &input.name, &url)?
        }
    };

    #[cfg(not(feature = "offline"))]
    let svg = fetch_svg_from_api(&input)?;

    Ok(quote! { #svg })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_input(input: &str) -> syn::Result<IconifyInput> {
        syn::parse_str::<IconifyInput>(input)
    }

    #[test]
    fn parse_basic() {
        let input = parse_input(r#""mdi:home""#).unwrap();
        assert_eq!(input.pack, "mdi");
        assert_eq!(input.name, "home");
        assert!(input.color.is_none());
    }

    #[test]
    fn parse_with_attributes() {
        let input =
            parse_input(r#""mdi:home", color = "red", width = "24", flip = "horizontal""#)
                .unwrap();
        assert_eq!(input.color.as_deref(), Some("red"));
        assert_eq!(input.width.as_deref(), Some("24"));
        assert!(matches!(input.flip, Some(IconifyFlip::Horizontal)));
    }

    #[test]
    fn parse_no_colon_fails() {
        let err = parse_input(r#""mdi-home""#).unwrap_err();
        assert!(err.to_string().contains("pack_name:icon_name"));
    }

    #[test]
    fn parse_too_many_colons_fails() {
        let err = parse_input(r#""mdi:home:extra""#).unwrap_err();
        assert!(err.to_string().contains("pack_name:icon_name"));
    }

    #[test]
    fn parse_unknown_attribute_fails() {
        let err = parse_input(r#""mdi:home", unknown = "value""#).unwrap_err();
        assert!(err.to_string().contains("unknown attribute"));
    }

    #[test]
    fn url_construction() {
        let input = parse_input(r#""mdi:home""#).unwrap();
        let url = input.icon_url().unwrap();
        assert!(url.contains("/mdi/home.svg"));
    }

    #[test]
    fn url_with_params() {
        let input = parse_input(r#""mdi:home", color = "red", width = "24""#).unwrap();
        let url = input.icon_url().unwrap();
        assert!(url.contains("color=red"));
        assert!(url.contains("width=24"));
    }

    #[test]
    fn fetch_existing_icon() {
        let input = parse_input(r#""mdi:home""#).unwrap();
        let svg = fetch_svg_from_api(&input).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn fetch_nonexistent_icon_suggests_alternatives() {
        let input = parse_input(r#""mdi:this-icon-surely-does-not-exist-xyz""#).unwrap();
        let err = fetch_svg_from_api(&input).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("icon not found"));
        assert!(msg.contains("iconify_cli"));
    }
}
