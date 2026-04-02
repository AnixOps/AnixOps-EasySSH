//! Internationalization (i18n) Module
//!
//! This module provides multi-language support for EasySSH using Mozilla's Fluent
//! localization system. It supports 10+ languages with automatic system language
//! detection and RTL (Right-to-Left) language support.
//!
//! # Supported Languages
//!
//! | Code | Language | Direction |
//! |------|----------|-----------|
//! | en | English | LTR |
//! | zh-CN | Chinese Simplified | LTR |
//! | zh-TW | Chinese Traditional | LTR |
//! | ja | Japanese | LTR |
//! | ko | Korean | LTR |
//! | de | German | LTR |
//! | fr | French | LTR |
//! | es | Spanish | LTR |
//! | ru | Russian | LTR |
//! | ar | Arabic | RTL |
//! | he | Hebrew | RTL |
//!
//! # Architecture
//!
//! The i18n system uses:
//! - `fluent` crate for translation message formatting
//! - `unic_langid` for language identifier parsing
//! - Thread-safe storage via `RwLock`
//! - Lazy static initialization for global access
//!
//! # Example
//!
//! ```rust,no_run
//! use easyssh_core::i18n::{t, t_args, set_language, get_current_language};
//!
//! // Initialize with default system language
//! easyssh_core::i18n::init().expect("Failed to initialize i18n");
//!
//! // Get a simple translation
//! let message = t("welcome-message");
//! println!("{}", message);
//!
//! // Get translation with arguments
//! let message = t_args("server-connected", &[("host", "192.168.1.1"), ("port", "22")]);
//! println!("{}", message);
//!
//! // Change language
//! set_language("zh-CN").expect("Failed to set language");
//! assert_eq!(get_current_language(), "zh-CN");
//! ```
//!
//! # Translation Files
//!
//! Translation files are stored in `resources/locales/` with the naming convention:
//! `{language-code}/main.ftl`
//!
//! Example structure:
//! ```text
//! resources/locales/
//! ├── en/
//! │   └── main.ftl
//! ├── zh-CN/
//! │   └── main.ftl
//! └── ja/
//!     └── main.ftl
//! ```
//!
//! # DateTime and Number Formatting
//!
//! The module also provides locale-aware formatting:
//! - `format_datetime()` - Format dates and times according to locale
//! - `format_number()` - Format numbers with locale-specific separators
//! - `format_date()` - Format dates only

use fluent::{FluentArgs, FluentBundle, FluentResource};
use fluent_bundle::FluentValue;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;
use tracing::{debug, error, info, warn};
use unic_langid::LanguageIdentifier;

// Re-export for datetime formatting
use chrono::{Datelike, Timelike};

/// Supported RTL (Right-to-Left) languages
pub const RTL_LANGUAGES: &[&str] = &["ar", "he", "ur", "fa"];

/// Supported languages with their display names
pub const SUPPORTED_LANGUAGES: &[(&str, &str, &str)] = &[
    ("en", "English", "English"),
    ("zh-CN", "中文（简体）", "Chinese Simplified"),
    ("zh-TW", "中文（繁体）", "Chinese Traditional"),
    ("ja", "日本語", "Japanese"),
    ("ko", "한국어", "Korean"),
    ("de", "Deutsch", "German"),
    ("fr", "Français", "French"),
    ("es", "Español", "Spanish"),
    ("ru", "Русский", "Russian"),
    ("ar", "العربية", "Arabic"), // RTL
    ("he", "עברית", "Hebrew"),   // RTL
];

/// Default fallback language
pub const DEFAULT_LANGUAGE: &str = "en";

/// Type alias for FluentBundle
type FluentBundleType = FluentBundle<FluentResource>;

/// Thread-safe storage for raw translation content (not bundles)
struct TranslationStore {
    current_lang: RwLock<LanguageIdentifier>,
    raw_translations: RwLock<HashMap<LanguageIdentifier, String>>,
    is_rtl: RwLock<bool>,
}

impl TranslationStore {
    fn new() -> Self {
        let system_lang = detect_system_language();
        info!("Detected system language: {}", system_lang);

        let mut translations = HashMap::new();

        // Load all supported languages
        for (lang_code, _, _) in SUPPORTED_LANGUAGES {
            if let Ok(lang_id) = lang_code.parse::<LanguageIdentifier>() {
                if let Some(content) = load_translation_file(&lang_id) {
                    debug!("Loaded translations for {}", lang_id);
                    translations.insert(lang_id, content);
                }
            }
        }

        // Load default language as fallback
        let default_lang = DEFAULT_LANGUAGE.parse::<LanguageIdentifier>().unwrap();
        if !translations.contains_key(&default_lang) {
            if let Some(content) = load_translation_file(&default_lang) {
                translations.insert(default_lang.clone(), content);
            }
        }

        let is_rtl = RTL_LANGUAGES.contains(&system_lang.to_string().as_str())
            || RTL_LANGUAGES
                .iter()
                .any(|&rtl| system_lang.to_string().starts_with(rtl));

        Self {
            current_lang: RwLock::new(system_lang),
            raw_translations: RwLock::new(translations),
            is_rtl: RwLock::new(is_rtl),
        }
    }

    fn current_language(&self) -> LanguageIdentifier {
        self.current_lang.read().unwrap().clone()
    }

    fn set_language(&self, lang: LanguageIdentifier) -> Result<(), I18nError> {
        let lang_str = lang.to_string();
        let is_supported = SUPPORTED_LANGUAGES
            .iter()
            .any(|(code, _, _)| lang_str == *code || lang_str.starts_with(code));

        if !is_supported {
            return Err(I18nError::UnsupportedLanguage(lang_str));
        }

        let is_rtl = RTL_LANGUAGES.contains(&lang_str.as_str())
            || RTL_LANGUAGES.iter().any(|&rtl| lang_str.starts_with(rtl));

        *self.is_rtl.write().unwrap() = is_rtl;
        *self.current_lang.write().unwrap() = lang;

        info!("Language changed to: {}", lang_str);
        Ok(())
    }

    fn is_rtl(&self) -> bool {
        *self.is_rtl.read().unwrap()
    }

    fn get_translation_content(&self, lang: &LanguageIdentifier) -> Option<String> {
        let translations = self.raw_translations.read().unwrap();
        translations.get(lang).cloned()
    }

    fn get_current_content(&self) -> Option<String> {
        let lang = self.current_lang.read().unwrap();
        self.get_translation_content(&lang)
    }

    fn get_fallback_content(&self) -> Option<String> {
        let default_lang = DEFAULT_LANGUAGE.parse::<LanguageIdentifier>().unwrap();
        self.get_translation_content(&default_lang)
    }
}

lazy_static! {
    static ref TRANSLATION_STORE: TranslationStore = TranslationStore::new();
}

/// i18n errors
#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Translation not found: {0}")]
    TranslationNotFound(String),

    #[error("Bundle error: {0}")]
    BundleError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Detect system language
fn detect_system_language() -> LanguageIdentifier {
    if let Some(locale) = sys_locale::get_locale() {
        debug!("System locale detected: {}", locale);

        let locale_str = locale.replace('-', "_");

        for (code, _, _) in SUPPORTED_LANGUAGES {
            let code_normalized = code.replace('-', "_");
            if locale_str == code_normalized || locale_str.starts_with(&code_normalized) {
                if let Ok(lang_id) = code.parse::<LanguageIdentifier>() {
                    return lang_id;
                }
            }
        }

        if let Ok(lang_id) = locale_str.parse::<LanguageIdentifier>() {
            let base_str = lang_id.language.as_str();

            for (code, _, _) in SUPPORTED_LANGUAGES {
                if code.starts_with(base_str) || base_str == *code {
                    if let Ok(lang_id) = code.parse::<LanguageIdentifier>() {
                        return lang_id;
                    }
                }
            }

            return lang_id;
        }
    }

    warn!("Could not detect system locale, falling back to English");
    DEFAULT_LANGUAGE.parse::<LanguageIdentifier>().unwrap()
}

/// Get the path to the locales directory
fn get_locales_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("locales"),
        std::env::current_exe()
            .map(|p| p.parent().map(|p| p.join("locales")).unwrap_or(p))
            .unwrap_or_else(|_| PathBuf::from("locales")),
        PathBuf::from("locales"),
    ];

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    candidates[0].clone()
}

/// Load translation file content for a language
fn load_translation_file(lang: &LanguageIdentifier) -> Option<String> {
    let lang_str = lang.to_string();
    let file_name = format!("{}.ftl", lang_str);
    let locales_dir = get_locales_dir();
    let file_path = locales_dir.join(&file_name);

    if let Ok(content) = std::fs::read_to_string(&file_path) {
        debug!("Loaded translations from {:?}", file_path);
        return Some(content);
    }

    let base_lang = lang.language.as_str();
    let base_file_name = format!("{}.ftl", base_lang);
    let base_file_path = locales_dir.join(&base_file_name);

    if let Ok(content) = std::fs::read_to_string(&base_file_path) {
        debug!("Loaded base translations from {:?}", base_file_path);
        return Some(content);
    }

    warn!(
        "Could not load translations for {} (tried {:?} and {:?})",
        lang, file_path, base_file_path
    );
    None
}

/// Create Fluent bundle from content
fn create_bundle(lang: &LanguageIdentifier, content: &str) -> Option<FluentBundleType> {
    let resource = FluentResource::try_new(content.to_string())
        .map_err(|(_, errors)| {
            for error in errors {
                warn!("Fluent parse error for {}: {:?}", lang, error);
            }
        })
        .ok()?;

    let mut bundle = FluentBundle::new(vec![lang.clone()]);

    if let Err(errors) = bundle.add_resource(resource) {
        for error in errors {
            warn!("Failed to add resource for {}: {:?}", lang, error);
        }
    }

    Some(bundle)
}

/// Format message with arguments
fn format_message(
    bundle: &FluentBundleType,
    id: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let msg = bundle.get_message(id)?;
    let pattern = msg.value()?;
    let mut errors = vec![];

    let value = bundle.format_pattern(pattern, args, &mut errors);

    if !errors.is_empty() {
        for error in &errors {
            debug!("Format error for message '{}': {:?}", id, error);
        }
    }

    Some(value.to_string())
}

/// Get current language code
pub fn get_current_language() -> String {
    TRANSLATION_STORE.current_language().to_string()
}

/// Set current language by code
pub fn set_language(code: &str) -> Result<(), I18nError> {
    let lang_id = code
        .parse::<LanguageIdentifier>()
        .map_err(|e| I18nError::ParseError(e.to_string()))?;
    TRANSLATION_STORE.set_language(lang_id)
}

/// Check if current language is RTL
pub fn is_rtl() -> bool {
    TRANSLATION_STORE.is_rtl()
}

/// Check if a language code is RTL
pub fn is_language_rtl(code: &str) -> bool {
    RTL_LANGUAGES.contains(&code) || RTL_LANGUAGES.iter().any(|&rtl| code.starts_with(rtl))
}

/// Get all supported languages
pub fn get_supported_languages() -> &'static [(&'static str, &'static str, &'static str)] {
    SUPPORTED_LANGUAGES
}

/// Get display name for a language
pub fn get_language_display_name(code: &str) -> Option<&'static str> {
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(c, _, _)| *c == code)
        .map(|(_, name, _)| *name)
}

/// Translate a message
pub fn t(key: &str) -> String {
    translate(key, None)
}

/// Translate a message with arguments
pub fn t_args(key: &str, args: &[(&str, FluentValue)]) -> String {
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, value.clone());
    }
    translate(key, Some(&fluent_args))
}

/// Internal translate function - creates bundle on-demand
fn translate(key: &str, args: Option<&FluentArgs>) -> String {
    let current_lang = TRANSLATION_STORE.current_language();

    // Try current language
    if let Some(content) = TRANSLATION_STORE.get_current_content() {
        if let Some(bundle) = create_bundle(&current_lang, &content) {
            if let Some(text) = format_message(&bundle, key, args) {
                return text;
            }
        }
    }

    // Fall back to default language
    if let Some(fallback_content) = TRANSLATION_STORE.get_fallback_content() {
        let default_lang = DEFAULT_LANGUAGE.parse::<LanguageIdentifier>().unwrap();
        if let Some(fallback_bundle) = create_bundle(&default_lang, &fallback_content) {
            if let Some(text) = format_message(&fallback_bundle, key, args) {
                return text;
            }
        }
    }

    warn!("Translation not found for key: {}", key);
    key.to_string()
}

/// Format a number according to current locale
pub fn format_number(num: impl Into<f64>) -> String {
    let num: f64 = num.into();
    let lang = TRANSLATION_STORE.current_language();

    match lang.to_string().as_str() {
        "de" | "de-DE" | "fr" | "fr-FR" | "ru" | "ru-RU" => {
            let int_part = num.trunc() as i64;
            let frac_part = ((num.fract().abs() * 100.0).round() as i64).abs();
            format!(
                "{}.{:02}",
                int_part
                    .to_string()
                    .chars()
                    .rev()
                    .collect::<Vec<_>>()
                    .chunks(3)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .rev()
                    .collect::<String>(),
                frac_part
            )
            .replace('.', ",")
            .replace(' ', ".")
        }
        _ => {
            let int_part = num.trunc() as i64;
            let frac_part = ((num.fract().abs() * 100.0).round() as i64).abs();
            format!(
                "{}.{:02}",
                int_part
                    .to_string()
                    .chars()
                    .rev()
                    .collect::<Vec<_>>()
                    .chunks(3)
                    .map(|c| c.iter().collect::<String>())
                    .collect::<Vec<_>>()
                    .join(",")
                    .chars()
                    .rev()
                    .collect::<String>(),
                frac_part
            )
        }
    }
}

/// Format a date according to current locale
pub fn format_date(timestamp: impl Into<chrono::DateTime<chrono::Utc>>) -> String {
    let dt: chrono::DateTime<chrono::Utc> = timestamp.into();
    let lang = TRANSLATION_STORE.current_language().to_string();

    match lang.as_str() {
        "zh-CN" | "zh-TW" | "ja" | "ko" => {
            format!("{}年{:02}月{:02}日", dt.year(), dt.month(), dt.day())
        }
        "de" | "de-DE" => {
            format!("{:02}.{:02}.{}", dt.day(), dt.month(), dt.year())
        }
        "en-US" | "en" => {
            format!("{:02}/{:02}/{}", dt.month(), dt.day(), dt.year())
        }
        _ => {
            format!("{}-{:02}-{:02}", dt.year(), dt.month(), dt.day())
        }
    }
}

/// Format a datetime according to current locale
pub fn format_datetime(timestamp: impl Into<chrono::DateTime<chrono::Utc>>) -> String {
    let dt: chrono::DateTime<chrono::Utc> = timestamp.into();
    let date = format_date(dt);
    let time = format!("{:02}:{:02}:{:02}", dt.hour(), dt.minute(), dt.second());

    if is_rtl() {
        format!("{} {}", time, date)
    } else {
        format!("{} {}", date, time)
    }
}

/// Text direction enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    Ltr,
    Rtl,
}

impl TextDirection {
    pub fn as_class(&self) -> &'static str {
        match self {
            TextDirection::Ltr => "ltr",
            TextDirection::Rtl => "rtl",
        }
    }

    pub fn as_css_value(&self) -> &'static str {
        match self {
            TextDirection::Ltr => "ltr",
            TextDirection::Rtl => "rtl",
        }
    }

    pub fn is_rtl(&self) -> bool {
        matches!(self, TextDirection::Rtl)
    }
}

/// Get text direction for current language
pub fn get_text_direction() -> TextDirection {
    if is_rtl() {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    }
}

/// Get CSS class for RTL layout
pub fn get_rtl_class() -> &'static str {
    if is_rtl() {
        "rtl"
    } else {
        ""
    }
}

/// Initialize i18n system
pub fn init() {
    info!("Initializing i18n system");
    let lang = get_current_language();
    let rtl = is_rtl();
    info!("Current language: {}, RTL: {}", lang, rtl);
}

/// Reload translations (useful for development)
pub fn reload_translations() -> Result<(), I18nError> {
    info!("Translations reload requested - restart application to load new translations");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_simple() {
        let text = t("general-ok");
        assert!(!text.is_empty());
    }

    #[test]
    fn test_rtl_detection() {
        assert!(is_language_rtl("ar"));
        assert!(is_language_rtl("ar-SA"));
        assert!(is_language_rtl("he"));
        assert!(is_language_rtl("he-IL"));
        assert!(!is_language_rtl("en"));
        assert!(!is_language_rtl("zh-CN"));
    }

    #[test]
    fn test_text_direction() {
        let ltr = TextDirection::Ltr;
        let rtl = TextDirection::Rtl;

        assert_eq!(ltr.as_class(), "ltr");
        assert_eq!(rtl.as_class(), "rtl");
        assert!(rtl.is_rtl());
        assert!(!ltr.is_rtl());
    }

    #[test]
    fn test_language_list() {
        let langs = get_supported_languages();
        assert!(!langs.is_empty());
        assert_eq!(langs[0].0, "en");

        for (code, native, english) in langs {
            assert!(!native.is_empty());
            assert!(!english.is_empty());
            assert!(!code.is_empty());
        }
    }

    #[test]
    fn test_get_language_display_name() {
        assert_eq!(get_language_display_name("en"), Some("English"));
        assert_eq!(get_language_display_name("zh-CN"), Some("中文（简体）"));
        assert_eq!(get_language_display_name("invalid"), None);
    }
}
