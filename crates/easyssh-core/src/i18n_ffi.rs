//! FFI bindings for internationalization
//!
//! These bindings allow native UI platforms (GTK4, WinUI, SwiftUI)
//! to access the Rust i18n system.

use crate::i18n as core_i18n;
use fluent_bundle::FluentValue;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// Initialize the i18n system
#[no_mangle]
pub extern "C" fn easyssh_i18n_init() {
    core_i18n::init();
}

/// Get current language code
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_get_language() -> *mut c_char {
    let lang = core_i18n::get_current_language();
    CString::new(lang).unwrap_or_default().into_raw()
}

/// Set current language
/// Returns 0 on success, -1 on error
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure that lang_code is a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_set_language(lang_code: *const c_char) -> c_int {
    if lang_code.is_null() {
        return -1;
    }

    let lang = unsafe {
        match CStr::from_ptr(lang_code).to_str() {
            Ok(s) => s,
            Err(_) => return -1,
        }
    };

    match core_i18n::set_language(lang) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Check if current language is RTL
/// Returns 1 if RTL, 0 otherwise
#[no_mangle]
pub extern "C" fn easyssh_i18n_is_rtl() -> c_int {
    if core_i18n::is_rtl() {
        1
    } else {
        0
    }
}

/// Check if a specific language is RTL
/// Returns 1 if RTL, 0 otherwise
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure that lang_code is a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_is_language_rtl(lang_code: *const c_char) -> c_int {
    if lang_code.is_null() {
        return 0;
    }

    let lang = unsafe {
        match CStr::from_ptr(lang_code).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        }
    };

    if core_i18n::is_language_rtl(lang) {
        1
    } else {
        0
    }
}

/// Translate a key
/// Returns a string that must be freed with easyssh_i18n_free_string
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure that key is a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_translate(key: *const c_char) -> *mut c_char {
    if key.is_null() {
        return CString::new("").unwrap_or_default().into_raw();
    }

    let key_str = unsafe {
        match CStr::from_ptr(key).to_str() {
            Ok(s) => s,
            Err(_) => return CString::new("").unwrap_or_default().into_raw(),
        }
    };

    let translated = core_i18n::t(key_str);
    CString::new(translated).unwrap_or_default().into_raw()
}

/// Translate with a single argument
/// Returns a string that must be freed with easyssh_i18n_free_string
///
/// # Safety
/// This function is unsafe because it dereferences raw pointers.
/// The caller must ensure that all pointer arguments are valid, null-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_translate_with_arg(
    key: *const c_char,
    arg_name: *const c_char,
    arg_value: *const c_char,
) -> *mut c_char {
    if key.is_null() || arg_name.is_null() || arg_value.is_null() {
        return CString::new("").unwrap_or_default().into_raw();
    }

    let key_str = unsafe {
        match CStr::from_ptr(key).to_str() {
            Ok(s) => s,
            Err(_) => return CString::new("").unwrap_or_default().into_raw(),
        }
    };

    let name_str = unsafe {
        match CStr::from_ptr(arg_name).to_str() {
            Ok(s) => s,
            Err(_) => return CString::new("").unwrap_or_default().into_raw(),
        }
    };

    let value_str = unsafe {
        match CStr::from_ptr(arg_value).to_str() {
            Ok(s) => s,
            Err(_) => return CString::new("").unwrap_or_default().into_raw(),
        }
    };

    let translated = core_i18n::t_args(key_str, &[(name_str, FluentValue::from(value_str))]);
    CString::new(translated).unwrap_or_default().into_raw()
}

/// Format a number for the current locale
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_format_number(num: f64) -> *mut c_char {
    let formatted = core_i18n::format_number(num);
    CString::new(formatted).unwrap_or_default().into_raw()
}

/// Format a timestamp (seconds since epoch) for the current locale
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_format_date(timestamp_secs: i64) -> *mut c_char {
    let dt = chrono::DateTime::from_timestamp(timestamp_secs, 0).unwrap_or_else(chrono::Utc::now);
    let formatted = core_i18n::format_date(dt);
    CString::new(formatted).unwrap_or_default().into_raw()
}

/// Format a timestamp (seconds since epoch) with time for the current locale
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_format_datetime(timestamp_secs: i64) -> *mut c_char {
    let dt = chrono::DateTime::from_timestamp(timestamp_secs, 0).unwrap_or_else(chrono::Utc::now);
    let formatted = core_i18n::format_datetime(dt);
    CString::new(formatted).unwrap_or_default().into_raw()
}

/// Get the text direction class
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_get_direction_class() -> *mut c_char {
    let class = if core_i18n::is_rtl() { "rtl" } else { "ltr" };
    CString::new(class).unwrap_or_default().into_raw()
}

/// Get the display name for a language
/// Returns a string that must be freed with easyssh_i18n_free_string
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure that lang_code is a valid, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_get_language_name(
    lang_code: *const c_char,
    _native: c_int,
) -> *mut c_char {
    if lang_code.is_null() {
        return CString::new("").unwrap_or_default().into_raw();
    }

    let lang = unsafe {
        match CStr::from_ptr(lang_code).to_str() {
            Ok(s) => s,
            Err(_) => return CString::new("").unwrap_or_default().into_raw(),
        }
    };

    if let Some(name) = core_i18n::get_language_display_name(lang) {
        CString::new(name).unwrap_or_default().into_raw()
    } else {
        CString::new(lang).unwrap_or_default().into_raw()
    }
}

/// Get the number of supported languages
#[no_mangle]
pub extern "C" fn easyssh_i18n_get_language_count() -> c_int {
    core_i18n::SUPPORTED_LANGUAGES.len() as c_int
}

/// Get language code at index
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_get_language_code(index: c_int) -> *mut c_char {
    let idx = index as usize;
    if idx >= core_i18n::SUPPORTED_LANGUAGES.len() {
        return CString::new("").unwrap_or_default().into_raw();
    }

    let lang = core_i18n::SUPPORTED_LANGUAGES[idx].0;
    CString::new(lang).unwrap_or_default().into_raw()
}

/// Free a string returned by the i18n functions
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer.
/// The caller must ensure that s is a valid pointer returned by one of the i18n functions,
/// or null.
#[no_mangle]
pub unsafe extern "C" fn easyssh_i18n_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

/// Get the default language code
/// Returns a string that must be freed with easyssh_i18n_free_string
#[no_mangle]
pub extern "C" fn easyssh_i18n_get_default_language() -> *mut c_char {
    CString::new(core_i18n::DEFAULT_LANGUAGE)
        .unwrap_or_default()
        .into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_init() {
        // Should not panic
        easyssh_i18n_init();
    }

    #[test]
    fn test_ffi_rtl_detection() {
        // English is not RTL
        let en = CString::new("en").unwrap();
        assert_eq!(unsafe { easyssh_i18n_is_language_rtl(en.as_ptr()) }, 0);

        // Arabic is RTL
        let ar = CString::new("ar").unwrap();
        assert_eq!(unsafe { easyssh_i18n_is_language_rtl(ar.as_ptr()) }, 1);

        // Hebrew is RTL
        let he = CString::new("he").unwrap();
        assert_eq!(unsafe { easyssh_i18n_is_language_rtl(he.as_ptr()) }, 1);
    }

    #[test]
    fn test_ffi_translate() {
        let key = CString::new("general-ok").unwrap();
        let result = unsafe { easyssh_i18n_translate(key.as_ptr()) };

        assert!(!result.is_null());

        // Clean up
        unsafe {
            easyssh_i18n_free_string(result);
        }
    }

    #[test]
    fn test_ffi_language_count() {
        let count = easyssh_i18n_get_language_count();
        assert!(count > 0);
    }

    #[test]
    fn test_ffi_get_language_code() {
        let code = easyssh_i18n_get_language_code(0);
        assert!(!code.is_null());

        // Clean up
        unsafe {
            easyssh_i18n_free_string(code);
        }
    }
}
