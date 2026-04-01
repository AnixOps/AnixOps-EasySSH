# EasySSH Internationalization (i18n) System

## Overview

EasySSH includes a comprehensive internationalization system supporting 11 languages including RTL (Right-to-Left) languages.

## Supported Languages

| Code | Language | Native Name | RTL |
|------|----------|-------------|-----|
| `en` | English | English | No |
| `zh-CN` | Chinese (Simplified) | 中文（简体） | No |
| `zh-TW` | Chinese (Traditional) | 中文（繁體） | No |
| `ja` | Japanese | 日本語 | No |
| `ko` | Korean | 한국어 | No |
| `de` | German | Deutsch | No |
| `fr` | French | Français | No |
| `es` | Spanish | Español | No |
| `ru` | Russian | Русский | No |
| `ar` | Arabic | العربية | **Yes** |
| `he` | Hebrew | עברית | **Yes** |

## Architecture

### Backend (Rust)

The Rust backend uses the [Fluent](https://projectfluent.org/) localization system:

- **fluent-bundle**: Core localization framework
- **fluent-syntax**: Parsing Fluent files
- **unic-langid**: Language identifier handling
- **intl-memoizer**: Caching for performance
- **sys-locale**: System language detection
- **rust-embed**: Embedding translation files

```rust
// Example usage in Rust
use easyssh_core::i18n::{t, t_args, format_date, is_rtl};

// Simple translation
let text = t("server-detail-connect");

// Translation with arguments
let text = t_args("connection-retrying", &[
    ("attempt", 1.into()),
    ("max", 3.into()),
]);

// Format dates
let date_text = format_date(chrono::Utc::now());

// Check RTL
let rtl = is_rtl();
```

### Frontend (React/TypeScript)

The React frontend uses Zustand for state management:

```typescript
// Using the i18n hook
import { useI18n, useRTL } from './hooks/useI18n';

function MyComponent() {
  const { t, formatDate, currentLanguage, isRTL } = useI18n();
  const { rtlClass, flexDirection } = useRTL();

  return (
    <div className={rtlClass}>
      <p>{t('server-detail-connect')}</p>
      <p>{formatDate(Date.now())}</p>
    </div>
  );
}
```

## Translation Files

Translations are stored in FTL (Fluent) format in `/locales/`:

```ftl
# locales/en.ftl
server-detail-connect = Connect
connection-retrying = Retrying... ({ $attempt }/{ $max })
```

### Key Naming Convention

```
{category}-{subcategory}-{element}

Examples:
- server-detail-connect
- terminal-new-tab
- settings-language
- error-connection-failed
```

## RTL Support

EasySSH provides comprehensive RTL support for Arabic and Hebrew:

### CSS RTL Utilities

```css
/* Auto-flip for RTL */
.rtl .flex-row {
  flex-direction: row-reverse;
}

/* Logical properties */
.rtl .ms-2 { margin-right: 0.5rem; }
.rtl .me-2 { margin-left: 0.5rem; }
```

### RTL Hook

```typescript
const {
  isRTL,
  direction,
  rtlClass,
  textAlign,
  flexDirection,
  cn, // Conditional class helper
  iconClass, // Flip icons
} = useRTL();
```

### RTL-Aware Components

All built-in components automatically adapt to RTL:
- Sidebar position flips
- Icon directions flip (where appropriate)
- Text alignment adjusts
- Margins/paddings use logical properties

## Language Selection

### Automatic Detection

System language is automatically detected on first launch:

```rust
// System detection in Rust
let lang = detect_system_language();
```

### Manual Selection

Users can change language in Settings:

```typescript
const { setLanguage, languages } = useLanguage();

<LanguageSelector
  size="md"
  showNativeName={true}
  showRTLIndicator={true}
/>
```

## Formatting

### Numbers

```typescript
const { formatNumber } = useI18n();

// US: 1,234.56
// DE: 1.234,56
formatNumber(1234.56);
```

### Dates

```typescript
const { formatDate, formatDateTime } = useI18n();

// Locale-aware formatting
formatDate(new Date());      // e.g., "12/31/2024" or "31.12.2024"
formatDateTime(new Date());  // e.g., "12/31/2024 14:30"
```

## Adding a New Language

1. Create a new FTL file in `/locales/`:
   ```bash
   cp locales/en.ftl locales/xx.ftl
   ```

2. Translate all keys in the file

3. Add to `SUPPORTED_LANGUAGES` in:
   - `core/src/i18n.rs`
   - `src/stores/i18nStore.ts`

4. Run conversion script:
   ```bash
   node scripts/convert-translations.js
   ```

5. Test RTL support if applicable

## Translation Management Integration

EasySSH supports integration with translation management platforms:

### Crowdin

```yaml
# crowdin.yml
project_id: YOUR_PROJECT_ID
api_token: YOUR_API_TOKEN
files:
  - source: /locales/en.ftl
    translation: /locales/%two_letters_code%.ftl
```

### Lokalise

```bash
# Upload translations
lokalise file upload --token <token> --project-id <id> locales/en.ftl

# Download translations
lokalise file download --token <token> --project-id <id> --format ftl
```

## Best Practices

### For Developers

1. **Always use translation keys** - Never hardcode strings
2. **Use namespaced keys** - e.g., `server-dialog-add-title`
3. **Include context in keys** - Makes translation easier
4. **Handle pluralization** - Use Fluent's plural forms
5. **Test in RTL mode** - Ensure layouts work correctly

### For Translators

1. **Keep placeholders** - Preserve `{ $variable }` syntax
2. **Match tone** - Maintain consistent formality level
3. **Test in app** - See translations in context
4. **Consider space** - Some languages expand text by 30%

### Testing i18n

```bash
# Run with specific language
LANG=de npm run dev

# Test RTL
LANG=ar npm run dev

# Check for untranslated strings
npm run i18n:check
```

## Troubleshooting

### Common Issues

1. **Translations not loading**
   - Check FTL file syntax
   - Verify conversion script ran
   - Check browser console for errors

2. **RTL layout issues**
   - Ensure `rtl.css` is imported
   - Check logical property usage
   - Test with real RTL content

3. **Dates/numbers not formatting**
   - Verify Intl API support in browser
   - Check locale strings are valid BCP 47 tags

## API Reference

### Rust (Backend)

| Function | Description |
|----------|-------------|
| `t(key)` | Translate a key |
| `t_args(key, args)` | Translate with arguments |
| `format_number(n)` | Format number for locale |
| `format_date(dt)` | Format date for locale |
| `format_datetime(dt)` | Format datetime for locale |
| `is_rtl()` | Check if current language is RTL |
| `set_language(code)` | Change language |

### TypeScript (Frontend)

| Hook | Description |
|------|-------------|
| `useI18n()` | Full i18n functionality |
| `useTranslation()` | Translation only |
| `useFormat()` | Formatting only |
| `useLanguage()` | Language management |
| `useRTL()` | RTL utilities |
| `useNamespace(ns)` | Namespaced translations |

## Contributing Translations

We welcome community translations! Please:

1. Fork the repository
2. Create a new branch: `i18n/add-language-XX`
3. Add translations to `/locales/XX.ftl`
4. Test thoroughly
5. Submit a pull request

For questions or issues, please open a GitHub issue.
