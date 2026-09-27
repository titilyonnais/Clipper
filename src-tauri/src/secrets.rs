//! API keys live in the Windows Credential Manager, never in the database
//! and never in the web view.

const SERVICE: &str = "com.clipper.app";

fn account(provider: &str) -> Option<&'static str> {
    match provider {
        "openai" => Some("openai_api_key"),
        "anthropic" => Some("anthropic_api_key"),
        _ => None,
    }
}

#[cfg(windows)]
fn entry(provider: &str) -> Result<keyring::Entry, String> {
    let user = account(provider).ok_or("Fournisseur inconnu.")?;
    keyring::Entry::new(SERVICE, user).map_err(|e| e.to_string())
}

#[cfg(windows)]
pub fn get(provider: &str) -> Option<String> {
    entry(provider)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.is_empty())
}

#[cfg(windows)]
pub fn set(provider: &str, key: &str) -> Result<(), String> {
    let entry = entry(provider)?;
    let key = key.trim();
    if key.is_empty() {
        return match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    entry.set_password(key).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub fn get(provider: &str) -> Option<String> {
    account(provider).and_then(|a| std::env::var(format!("CLIPPER_{}", a.to_uppercase())).ok())
}

#[cfg(not(windows))]
pub fn set(_provider: &str, _key: &str) -> Result<(), String> {
    let _ = SERVICE;
    Err("Stockage sécurisé des clés disponible uniquement sous Windows.".into())
}

pub fn is_set(provider: &str) -> bool {
    get(provider).is_some()
}
