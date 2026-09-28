//! API keys live in the Windows Credential Manager, never in the database
//! and never in the web view.

const SERVICE: &str = "com.clipper.app";

fn entry(provider: &str) -> Result<keyring::Entry, String> {
    let user = match provider {
        "openai" => "openai_api_key",
        "anthropic" => "anthropic_api_key",
        _ => return Err("Fournisseur inconnu.".into()),
    };
    keyring::Entry::new(SERVICE, user).map_err(|e| e.to_string())
}

pub fn get(provider: &str) -> Option<String> {
    entry(provider)
        .ok()?
        .get_password()
        .ok()
        .filter(|k| !k.is_empty())
}

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

pub fn is_set(provider: &str) -> bool {
    get(provider).is_some()
}
