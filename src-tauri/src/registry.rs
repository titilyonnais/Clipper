//! Minimal access to the current user's registry hive.

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_BINARY, REG_DWORD, REG_EXPAND_SZ,
    REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
};

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// An open key of `HKEY_CURRENT_USER`, created if missing.
pub struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { RegCloseKey(self.0) };
    }
}

impl Key {
    pub fn open(path: &str, write: bool) -> Result<Key, String> {
        let mut hkey: HKEY = std::ptr::null_mut();
        let access = if write { KEY_READ | KEY_WRITE } else { KEY_READ };
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(path).as_ptr(),
                0,
                std::ptr::null(),
                REG_OPTION_NON_VOLATILE,
                access,
                std::ptr::null(),
                &mut hkey,
                std::ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Registre inaccessible (erreur {status})."));
        }
        Ok(Key(hkey))
    }

    /// Raw data and type of a value, `None` when absent or unreadable.
    fn raw(&self, name: &str) -> Option<(REG_VALUE_TYPE, Vec<u8>)> {
        let name = wide(name);
        let (mut kind, mut size) = (0u32, 0u32);
        // First the size, then the data: values have no fixed length.
        let status = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let status = unsafe {
            RegQueryValueExW(
                self.0,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buf.as_mut_ptr(),
                &mut size,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        buf.truncate(size as usize);
        Some((kind, buf))
    }

    pub fn string(&self, name: &str) -> Option<String> {
        let (kind, data) = self.raw(name)?;
        if kind != REG_SZ && kind != REG_EXPAND_SZ {
            return None;
        }
        let units: Vec<u16> = data
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        Some(String::from_utf16_lossy(&units).trim_end_matches('\0').to_string())
    }

    pub fn binary(&self, name: &str) -> Option<Vec<u8>> {
        self.raw(name).map(|(_, data)| data)
    }

    fn set(&self, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> Result<(), String> {
        let status = unsafe {
            RegSetValueExW(
                self.0,
                wide(name).as_ptr(),
                0,
                kind,
                data.as_ptr(),
                data.len() as u32,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Écriture dans le registre impossible (erreur {status})."));
        }
        Ok(())
    }

    pub fn set_string(&self, name: &str, value: &str) -> Result<(), String> {
        let data: Vec<u8> = wide(value).iter().flat_map(|u| u.to_le_bytes()).collect();
        self.set(name, REG_SZ, &data)
    }

    pub fn set_binary(&self, name: &str, value: &[u8]) -> Result<(), String> {
        self.set(name, REG_BINARY, value)
    }

    pub fn set_dword(&self, name: &str, value: u32) -> Result<(), String> {
        self.set(name, REG_DWORD, &value.to_le_bytes())
    }

    /// Delete a value; a value that does not exist is not an error.
    pub fn delete(&self, name: &str) -> Result<(), String> {
        let status = unsafe { RegDeleteValueW(self.0, wide(name).as_ptr()) };
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            return Err(format!("Écriture dans le registre impossible (erreur {status})."));
        }
        Ok(())
    }
}
