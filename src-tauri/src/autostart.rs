use std::iter;

use windows::core::PCWSTR;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
};

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(iter::once(0)).collect()
}

fn open_run_key(access: REG_SAM_FLAGS) -> Result<HKEY, String> {
    let subkey = to_wide(RUN_KEY_PATH);
    let mut hkey = HKEY::default();
    let err = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            Some(0),
            access,
            &mut hkey,
        )
    };
    if err == WIN32_ERROR(0) {
        Ok(hkey)
    } else {
        Err(format!("RegOpenKeyExW failed: 0x{:08x}", err.0))
    }
}

pub fn is_enabled(value_name: &str) -> Result<bool, String> {
    let name = to_wide(value_name);
    let hkey = open_run_key(KEY_QUERY_VALUE)?;
    let mut typ = REG_VALUE_TYPE(0);
    let mut len: u32 = 0;

    let err = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(name.as_ptr()),
            None,
            Some(&mut typ as *mut REG_VALUE_TYPE),
            None,
            Some(&mut len as *mut u32),
        )
    };
    unsafe {
        let _ = RegCloseKey(hkey);
    }

    // ERROR_FILE_NOT_FOUND
    if err == WIN32_ERROR(2) {
        return Ok(false);
    }

    if err != WIN32_ERROR(0) {
        return Err(format!("RegQueryValueExW failed: 0x{:08x}", err.0));
    }

    Ok(typ == REG_SZ && len > 1)
}

pub fn set_enabled(value_name: &str, enabled: bool, exe_command: &str) -> Result<(), String> {
    let name = to_wide(value_name);
    let hkey = open_run_key(KEY_SET_VALUE)?;

    let res = if enabled {
        // Store as REG_SZ (UTF-16) with a terminating NUL.
        let wide = to_wide(exe_command);
        let bytes = unsafe {
            std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2)
        };
        let err = unsafe {
            RegSetValueExW(
                hkey,
                PCWSTR(name.as_ptr()),
                Some(0),
                REG_SZ,
                Some(bytes),
            )
        };
        if err != WIN32_ERROR(0) {
            Err(format!("RegSetValueExW failed: 0x{:08x}", err.0))
        } else {
            Ok(())
        }
    } else {
        let err = unsafe { RegDeleteValueW(hkey, PCWSTR(name.as_ptr())) };
        // ERROR_FILE_NOT_FOUND is fine.
        if err != WIN32_ERROR(0) && err != WIN32_ERROR(2) {
            Err(format!("RegDeleteValueW failed: 0x{:08x}", err.0))
        } else {
            Ok(())
        }
    };

    unsafe {
        let _ = RegCloseKey(hkey);
    }
    res
}
