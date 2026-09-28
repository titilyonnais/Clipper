//! Content classification and previews for captured text.

use sha2::{Digest, Sha256};

pub const PREVIEW_CHARS: usize = 280;

pub fn hash_text(s: &str) -> String {
    format!("t:{:x}", Sha256::digest(s.as_bytes()))
}

pub fn hash_files(paths: &[String]) -> String {
    format!("f:{:x}", Sha256::digest(paths.join("\n").as_bytes()))
}

pub fn hash_image(png: &[u8]) -> String {
    format!("i:{:x}", Sha256::digest(png))
}

pub fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    // Signature (8) + IHDR length/type (8) + width (4) + height (4).
    if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(png[20..24].try_into().ok()?);
    Some((w, h))
}

pub fn make_preview(text: &str) -> String {
    let collapsed: String = text.trim().chars().take(PREVIEW_CHARS + 1).collect();
    if collapsed.chars().count() > PREVIEW_CHARS {
        let mut s: String = collapsed.chars().take(PREVIEW_CHARS).collect();
        s.push('…');
        s
    } else {
        collapsed
    }
}

pub fn files_preview(paths: &[String]) -> String {
    let names: Vec<&str> = paths
        .iter()
        .take(3)
        .map(|p| {
            std::path::Path::new(p)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(p)
        })
        .collect();
    match paths.len() {
        0 => String::new(),
        1..=3 => names.join(", "),
        n => format!("{} (+{} autres)", names.join(", "), n - 3),
    }
}

/// Returns (kind, language).
pub fn classify(text: &str) -> (&'static str, Option<&'static str>) {
    let t = text.trim();
    let lower_start: String = t.chars().take(12).collect::<String>().to_ascii_lowercase();
    if ["http://", "https://", "ftp://", "mailto:"]
        .iter()
        .any(|p| lower_start.starts_with(p))
        && !t.contains(char::is_whitespace)
    {
        return ("url", None);
    }
    match detect_language(t) {
        Some(lang) => ("code", Some(lang)),
        None => ("text", None),
    }
}

fn detect_language(t: &str) -> Option<&'static str> {
    // Heuristics are cheap but only worth running on text that looks structured.
    let sample: String = t.chars().take(4000).collect();
    let lower = sample.to_ascii_lowercase();
    let lines = sample.lines().count();
    let starts = |p: &str| lower.starts_with(p);
    let has = |p: &str| lower.contains(p);

    let bracketed = (starts("{") && t.ends_with('}')) || (starts("[") && t.ends_with(']'));
    if bracketed && (sample.contains("\":") || serde_json::from_str::<serde_json::Value>(t).is_ok())
    {
        return Some("json");
    }
    if starts("<") && has("</") {
        return Some(if has("<!doctype html") || has("<html") || has("<div") {
            "html"
        } else {
            "xml"
        });
    }
    if starts("#!/") || starts("$ ") || has("sudo ") && lines > 1 {
        return Some("bash");
    }
    let sql_start = [
        "select ",
        "insert into ",
        "update ",
        "delete from ",
        "create table ",
        "with ",
    ];
    if sql_start.iter().any(|p| starts(p))
        && (has(" from ") || has(" set ") || has(" values") || has("("))
    {
        return Some("sql");
    }
    if has("fn ") && (has("let ") || has("->") || has("pub ")) || has("impl ") && has("{") {
        return Some("rust");
    }
    if (has("def ") || has("class ")) && sample.contains(":\n")
        || starts("import ") && !has(";")
        || starts("from ") && has(" import ")
    {
        return Some("python");
    }
    if has("interface ") && has("{") || has(": string") || has(": number") || has("export type ") {
        return Some("typescript");
    }
    if has("function ") && has("{")
        || has("const ") && has(" = ")
        || has("=> {")
        || has("console.log")
    {
        return Some("javascript");
    }
    if has("{") && (has("color:") || has("display:") || has("margin:") || has("padding:")) {
        return Some("css");
    }
    if has("$env:") || has("get-childitem") || has("write-host") {
        return Some("powershell");
    }
    let indented = sample
        .lines()
        .filter(|l| l.starts_with("    ") || l.starts_with('\t'))
        .count();
    let semis = sample.matches(';').count();
    if lines >= 3 && indented >= 2 && (semis >= 3 || (has("{") && has("}"))) {
        return Some("plaintext");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_content() {
        assert_eq!(classify("https://example.com/a?b=c"), ("url", None));
        assert_eq!(classify("see https://example.com"), ("text", None));
        assert_eq!(classify("{\"a\": 1}"), ("code", Some("json")));
        assert_eq!(
            classify("SELECT id FROM users WHERE x = 1"),
            ("code", Some("sql"))
        );
        assert_eq!(classify("Bonjour, à demain !"), ("text", None));
        assert_eq!(
            classify("Quand je suis sur l'interface d'un client : const et class"),
            ("text", None)
        );
        assert_eq!(
            classify("fn main() {\n    let x = 1;\n}"),
            ("code", Some("rust"))
        );
    }

    #[test]
    fn preview_is_char_safe() {
        let s = "é".repeat(400);
        let p = make_preview(&s);
        assert_eq!(p.chars().count(), PREVIEW_CHARS + 1);
        assert!(p.ends_with('…'));
    }

    #[test]
    fn files_preview_summarizes() {
        let paths: Vec<String> = (1..=5).map(|i| format!("C:\\dir\\f{i}.txt")).collect();
        assert_eq!(files_preview(&paths), "f1.txt, f2.txt, f3.txt (+2 autres)");
    }

    #[test]
    fn png_dimensions_reads_header() {
        let png = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
        )
        .unwrap();
        assert_eq!(png_dimensions(&png), Some((1, 1)));
        assert_eq!(png_dimensions(b"nope"), None);
    }
}
