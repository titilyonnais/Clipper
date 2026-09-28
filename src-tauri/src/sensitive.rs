//! Detection of secrets in copied text. Flagged clips are masked in the UI
//! and kept out of the search index; they are never sent anywhere.
//!
//! Detection favours precision: a false alarm only masks a clip, but flagging
//! ordinary code identifiers or git hashes would be irritating.

const MAX_LEN: usize = 8 * 1024;

/// Known token prefixes and the minimum total length they come with.
const KEY_PREFIXES: &[(&str, usize)] = &[
    ("sk-ant-", 40),
    ("sk-proj-", 40),
    ("sk-", 32),
    ("sk_live_", 24),
    ("rk_live_", 24),
    ("ghp_", 36),
    ("gho_", 36),
    ("ghu_", 36),
    ("ghs_", 36),
    ("ghr_", 36),
    ("github_pat_", 40),
    ("glpat-", 24),
    ("xoxb-", 24),
    ("xoxp-", 24),
    ("xoxa-", 24),
    ("xapp-", 24),
    ("AKIA", 20),
    ("ASIA", 20),
    ("AIza", 39),
    ("npm_", 36),
    ("hf_", 34),
    ("shpat_", 32),
    ("SG.", 60),
];

const SECRET_KEYS: &[&str] = &[
    "password",
    "passwd",
    "pwd",
    "mot_de_passe",
    "motdepasse",
    "secret",
    "api_key",
    "apikey",
    "api-key",
    "access_token",
    "auth_token",
    "token",
    "private_key",
    "client_secret",
];

/// Returns a short French label when `text` looks like a secret.
pub fn detect(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.is_empty() || t.len() > MAX_LEN {
        return None;
    }
    if t.contains("-----BEGIN") && t.contains("PRIVATE KEY-----") {
        return Some("Clé privée");
    }
    if let Some(label) = secret_assignment(t) {
        return Some(label);
    }
    if t.chars().any(char::is_whitespace) {
        return card_number(t).then_some("Carte bancaire");
    }
    if KEY_PREFIXES
        .iter()
        .any(|(p, min)| t.starts_with(p) && t.len() >= *min && is_token_charset(t))
    {
        return Some("Clé d'API");
    }
    if is_jwt(t) {
        return Some("Jeton");
    }
    if card_number(t) {
        return Some("Carte bancaire");
    }
    if looks_like_password(t) {
        return Some("Mot de passe probable");
    }
    if looks_like_random_token(t) {
        return Some("Jeton");
    }
    None
}

fn is_token_charset(t: &str) -> bool {
    t.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// `password = hunter2`, `API_KEY: abc…` or `"token": "…"` lines (.env, JSON, YAML).
fn secret_assignment(t: &str) -> Option<&'static str> {
    for line in t.lines().take(200) {
        let lower = line.to_ascii_lowercase();
        let Some(pos) = lower.find([':', '=']) else {
            continue;
        };
        let key = lower[..pos].trim().trim_matches(['"', '\'']).trim();
        let key = key.rsplit(['.', ' ']).next().unwrap_or(key);
        let value = line[pos + 1..]
            .trim()
            .trim_matches(['"', '\'', ',', ';'])
            .trim();
        let named_secret = SECRET_KEYS.iter().any(|k| {
            key == *k || key.ends_with(&format!("_{k}")) || key.ends_with(&format!("-{k}"))
        });
        // `process.env.PASSWORD`, `getToken()`: a reference in code, not a value.
        let code_reference = value.contains('(')
            || value.contains('.')
                && value
                    .chars()
                    .all(|c| c.is_ascii_alphabetic() || c == '_' || c == '.');
        if named_secret
            && value.len() >= 6
            && !code_reference
            && !value.contains(char::is_whitespace)
            && !value.starts_with(['$', '{', '<', '('])
            && !matches!(value, "null" | "none" | "true" | "false" | "undefined")
        {
            return Some("Identifiants");
        }
    }
    None
}

fn is_jwt(t: &str) -> bool {
    let parts: Vec<&str> = t.split('.').collect();
    parts.len() == 3
        && t.starts_with("eyJ")
        && parts.iter().all(|p| {
            p.len() >= 10
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
}

/// 13 to 19 digits (spaces or dashes allowed) passing the Luhn check.
fn card_number(t: &str) -> bool {
    if !t
        .chars()
        .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
    {
        return false;
    }
    let digits: Vec<u32> = t.chars().filter_map(|c| c.to_digit(10)).collect();
    if !(13..=19).contains(&digits.len()) || digits.iter().all(|d| *d == digits[0]) {
        return false;
    }
    // Issuer prefixes (Visa, Mastercard, Amex, Discover): long numeric ids
    // (Discord, orders…) must not look like cards.
    let prefix: String = t.chars().filter(char::is_ascii_digit).take(4).collect();
    let p2: u32 = prefix[..2].parse().unwrap_or(0);
    let p4: u32 = prefix.parse().unwrap_or(0);
    let issuer = match digits.len() {
        16 => {
            prefix.starts_with('4')
                || (51..=55).contains(&p2)
                || (2221..=2720).contains(&p4)
                || prefix.starts_with("6011")
                || p2 == 65
        }
        15 => p2 == 34 || p2 == 37,
        13 | 19 => prefix.starts_with('4'),
        _ => false,
    };
    if !issuer {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let x = d * 2;
                if x > 9 {
                    x - 9
                } else {
                    x
                }
            } else {
                d
            }
        })
        .sum();
    sum % 10 == 0
}

/// 10 to 64 characters mixing letters, digits and at least one symbol, with
/// no sign of being a URL, path, e-mail address or code.
fn looks_like_password(t: &str) -> bool {
    let n = t.chars().count();
    if !(10..=64).contains(&n) {
        return false;
    }
    if t.contains("://")
        || t.contains('\\')
        || t.contains('/')
        || t.contains('@') && t.contains('.')
    {
        return false;
    }
    if t.contains("()")
        || t.contains("=>")
        || t.starts_with('#')
        || t.starts_with('<')
        || t.ends_with(':')
    {
        return false;
    }
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    // Dates, times, phone numbers and amounts are mostly digits.
    if letters < 3 || digits * 2 > n {
        return false;
    }
    let lower = t.chars().any(|c| c.is_lowercase());
    let upper = t.chars().any(|c| c.is_uppercase());
    let digit = digits > 0;
    let symbol = t
        .chars()
        .any(|c| !c.is_alphanumeric() && !matches!(c, '_' | '-' | '.'));
    symbol && digit && (lower || upper) && (lower && upper || n >= 14) && entropy(t) >= 3.0
}

/// Long random-looking tokens (base64-like), excluding plain hex which is
/// usually a hash or a commit id.
fn looks_like_random_token(t: &str) -> bool {
    if t.len() < 32 || t.len() > 512 || !is_token_charset(&t.replace(['+', '/', '='], "")) {
        return false;
    }
    if t.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return false;
    }
    let digit = t.chars().any(|c| c.is_ascii_digit());
    let lower = t.chars().any(|c| c.is_ascii_lowercase());
    let upper = t.chars().any(|c| c.is_ascii_uppercase());
    digit && lower && upper && entropy(t) >= 4.3
}

/// Shannon entropy in bits per character.
fn entropy(t: &str) -> f64 {
    let mut counts = std::collections::HashMap::new();
    let mut n = 0.0;
    for c in t.chars() {
        *counts.entry(c).or_insert(0.0) += 1.0;
        n += 1.0;
    }
    counts
        .values()
        .map(|c: &f64| {
            let p = c / n;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::detect;

    #[test]
    fn flags_secrets() {
        assert_eq!(
            detect("sk-ant-api03-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcd"),
            Some("Clé d'API")
        );
        assert_eq!(
            detect("ghp_1234567890abcdefghijklmnopqrstuvwxyz"),
            Some("Clé d'API")
        );
        assert_eq!(detect("AKIAIOSFODNN7EXAMPLE"), Some("Clé d'API"));
        assert_eq!(
            detect("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U"),
            Some("Jeton")
        );
        assert_eq!(detect("4970 1012 3456 7890"), None, "fails Luhn");
        assert_eq!(detect("4111 1111 1111 1111"), Some("Carte bancaire"));
        assert_eq!(detect("Tr0ub4dor&3xyz"), Some("Mot de passe probable"));
        assert_eq!(
            detect("DB_PASSWORD=s3cr3t-value\nDB_HOST=localhost"),
            Some("Identifiants")
        );
        assert_eq!(
            detect("\"api_key\": \"abcdef123456\""),
            Some("Identifiants")
        );
        assert_eq!(
            detect("-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n-----END OPENSSH PRIVATE KEY-----"),
            Some("Clé privée")
        );
        assert_eq!(
            detect("Xk9mP2vL8qR4tZ7wN3bH6jF1cD5sA0gYeUiOp"),
            Some("Jeton")
        );
    }

    #[test]
    fn ignores_ordinary_content() {
        for text in [
            "Bonjour, à demain !",
            "getUserById2",
            "3f786850e387550fdab836ed7e6dc881de23001b",
            "https://example.com/a?b=c&d=e1",
            "C:\\Users\\gilbe\\Desktop",
            "jean.dupont@example.com",
            "const password = process.env.PASSWORD;",
            "password: ${DB_PASSWORD}",
            "0612345678",
            "Le mot de passe est dans le coffre.",
            "fn main() { let x = 1; }",
            "2026-09-28T10:00:00Z",
            "Observateur#1:",
            "1545744893012345678",
            "4970 1012 3456 7891 234",
        ] {
            assert_eq!(detect(text), None, "{text}");
        }
    }
}
