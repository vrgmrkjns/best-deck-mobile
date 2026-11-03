//! Key icons on a 24px grid, stroked. Kept in step with `src/icons.rs` in best-deck-windows.

pub const ICONS: &[(&str, &str)] = &[
    ("dot", r#"<circle cx="12" cy="12" r="3"/>"#),
    ("plus", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("mic", r#"<path d="M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3"/>"#),
    ("mic-off", r#"<path d="M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3z"/><path d="M5 11a7 7 0 0 0 14 0M12 18v3M4 4l16 16"/>"#),
    ("headphones", r#"<path d="M4 14v-2a8 8 0 0 1 16 0v2"/><rect x="3" y="14" width="4" height="6" rx="1.5"/><rect x="17" y="14" width="4" height="6" rx="1.5"/>"#),
    ("hash", r#"<path d="M9 4L7 20M17 4l-2 16M4 9h16M3 15h16"/>"#),
    ("record", r#"<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4"/>"#),
    ("broadcast", r#"<circle cx="12" cy="12" r="2"/><path d="M7.8 16.2a6 6 0 0 1 0-8.4M16.2 7.8a6 6 0 0 1 0 8.4M5 19a10 10 0 0 1 0-14M19 5a10 10 0 0 1 0 14"/>"#),
    ("scene", r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 10h18"/>"#),
    ("volume", r#"<path d="M4 9v6h4l5 4V5L8 9H4z"/><path d="M16.5 8.5a5 5 0 0 1 0 7"/>"#),
    ("volume-off", r#"<path d="M4 9v6h4l5 4V5L8 9H4zM17 9l5 6M22 9l-5 6"/>"#),
    ("previous", r#"<path d="M18 5v14L8 12zM6 5v14"/>"#),
    ("play", r#"<path d="M7 4v16l13-8z"/>"#),
    ("next", r#"<path d="M6 5v14l10-7zM18 5v14"/>"#),
    ("stop", r#"<rect x="6" y="6" width="12" height="12" rx="1.5"/>"#),
    ("music", r#"<path d="M9 18V6l10-2v12"/><circle cx="6.5" cy="18" r="2.5"/><circle cx="16.5" cy="16" r="2.5"/>"#),
    ("save", r#"<path d="M5 4h11l3 3v13H5z"/><path d="M8 4v5h7V4M8 20v-6h8v6"/>"#),
    ("camera", r#"<rect x="3" y="7" width="18" height="13" rx="2"/><circle cx="12" cy="13.5" r="3.5"/><path d="M8 7l1.5-3h5L16 7"/>"#),
    ("clip", r#"<rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8 4v16M16 4v16M4 12h16"/>"#),
    ("folder", r#"<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>"#),
    ("lock", r#"<rect x="5" y="11" width="14" height="9" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>"#),
    ("monitor", r#"<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M9 20h6M12 16v4"/>"#),
    ("swap", r#"<path d="M4 8h14l-3-3M20 16H6l3 3"/>"#),
    ("sliders", r#"<path d="M6 4v16M12 4v16M18 4v16"/><rect x="4" y="13" width="4" height="3" rx="1"/><rect x="10" y="7" width="4" height="3" rx="1"/><rect x="16" y="11" width="4" height="3" rx="1"/>"#),
    ("keyboard", r#"<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10"/>"#),
    ("text", r#"<path d="M5 6h14M12 6v13M9 19h6"/>"#),
    ("app", r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 9h18M7 6.5h.01"/>"#),
    ("clock", r#"<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>"#),
    ("bolt", r#"<path d="M13 3L5 13h6l-1 8 8-10h-6z"/>"#),
    ("light", r#"<path d="M9 18h6M10 21h4M8 14a6 6 0 1 1 8 0c-.7.7-1 1.5-1 2.500H9c0-1-.3-1.800-1-2.500z"/>"#),
    ("power", r#"<path d="M12 3v9M6.300 6.700a8 8 0 1 0 11.400 0"/>"#),
    ("star", r#"<path d="M12 3l2.700 5.600 6.100.800-4.500 4.300 1.100 6.100L12 16.900 6.600 19.800l1.100-6.100L3.200 9.400l6.100-.800z"/>"#),
    ("globe", r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18"/>"#),
];

/// Prefix of a key icon that is a picture added on the PC, such as `custom:0f3a9c2b11d4e5f6.png`.
pub const CUSTOM: &str = "custom:";

/// The file name of a custom icon, if `icon` is one and well-formed: `<16 hex digits>.svg|png|gif`.
pub fn custom_file(icon: &str) -> Option<&str> {
    let file = icon.strip_prefix(CUSTOM)?;
    let (stem, ext) = file.split_once('.')?;
    let ok = stem.len() == 16 && stem.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) && matches!(ext, "svg" | "png" | "gif");
    ok.then_some(file)
}

/// The body of `name`, or the dot for an unknown name.
pub fn body(name: &str) -> &'static str {
    ICONS.iter().find(|(n, _)| *n == name).unwrap_or(&ICONS[0]).1
}

/// A complete SVG document for `name`, stroked in `color` (`#RRGGBB`).
pub fn svg(name: &str, color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#,
        body(name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_unique_and_unknown_falls_back() {
        let mut names: Vec<_> = ICONS.iter().map(|(n, _)| *n).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), ICONS.len());
        assert_eq!(body("no-such-icon"), body("dot"));
        assert!(svg("mic", "#FFFFFF").contains("stroke=\"#FFFFFF\""));
    }
}
