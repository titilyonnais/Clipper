//! Snippet variables: `{date}`, `{heure}`, `{jour}`, `{presse-papiers}`.
//! Unknown `{…}` sequences are left untouched (snippets often contain code).

use chrono::{Datelike, Local, Timelike};

const DAYS: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];

pub fn expand(template: &str, clipboard: impl Fn() -> Option<String>) -> String {
    if !template.contains('{') {
        return template.to_string();
    }
    let now = Local::now();
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let value = match &after[..end] {
            "date" => Some(format!(
                "{:02}/{:02}/{}",
                now.day(),
                now.month(),
                now.year()
            )),
            "heure" => Some(format!("{:02}:{:02}", now.hour(), now.minute())),
            "jour" => Some(DAYS[now.weekday().num_days_from_monday() as usize].to_string()),
            "presse-papiers" => Some(clipboard().unwrap_or_default()),
            _ => None,
        };
        match value {
            Some(v) => out.push_str(&v),
            None => out.push_str(&rest[start..start + end + 2]),
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::expand;

    #[test]
    fn expands_known_variables_only() {
        let out = expand("Le {date} à {heure} : {presse-papiers} {x} {", || {
            Some("copié".into())
        });
        assert!(out.starts_with("Le "));
        assert!(out.contains(" : copié {x} {"), "{out}");
        assert!(!out.contains("{date}") && !out.contains("{heure}"));
        assert_eq!(expand("fn a() { b }", || None), "fn a() { b }");
        assert_eq!(expand("sans variable", || None), "sans variable");
    }
}
