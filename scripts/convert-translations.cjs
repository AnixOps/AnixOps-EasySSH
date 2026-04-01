/**
 * Script to convert Fluent (.ftl) translation files to JSON for frontend use
 *
 * Run: node scripts/convert-translations.cjs
 */

const fs = require('fs');
const path = require('path');

const LOCALES_DIR = path.join(__dirname, '..', 'locales');
const OUTPUT_DIR = path.join(__dirname, '..', 'public', 'locales');

/**
 * Parse a Fluent message file
 * Handles:
 * - Simple messages: key = value
 * - Multi-line messages
 * - Comments (lines starting with #)
 * - Term references (lines starting with -)
 * - Placeholders: { $variable }
 */
function parseFTL(content) {
  const translations = {};
  const lines = content.split('\n');
  let currentKey = null;
  let currentValue = '';

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trim();

    // Skip empty lines and comments
    if (!trimmedLine || trimmedLine.startsWith('#')) {
      // Save previous entry if exists
      if (currentKey && currentValue.trim()) {
        translations[currentKey] = currentValue.trim();
      }
      currentKey = null;
      currentValue = '';
      continue;
    }

    // Check if this is a new key (starts with identifier and =)
    // Also handle term references (starting with -)
    const keyMatch = trimmedLine.match(/^(-?[a-zA-Z][a-zA-Z0-9_-]*)\s*=(.*)$/);

    if (keyMatch) {
      // Save previous entry if exists
      if (currentKey && currentValue.trim()) {
        translations[currentKey] = currentValue.trim();
      }

      // Remove leading dash from term references
      currentKey = keyMatch[1].startsWith('-') ? keyMatch[1].slice(1) : keyMatch[1];
      currentValue = keyMatch[2].trim();
    } else if (currentKey) {
      // Continuation of previous value
      currentValue += ' ' + trimmedLine;
    }
  }

  // Save last entry
  if (currentKey && currentValue.trim()) {
    translations[currentKey] = currentValue.trim();
  }

  return translations;
}

/**
 * Convert all .ftl files to .json
 */
function convertTranslations() {
  console.log('🌍 Converting Fluent translations to JSON...\n');

  // Ensure output directory exists
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  }

  // Get all .ftl files
  const ftlFiles = fs.readdirSync(LOCALES_DIR)
    .filter(file => file.endsWith('.ftl'));

  if (ftlFiles.length === 0) {
    console.error('❌ No .ftl files found in', LOCALES_DIR);
    process.exit(1);
  }

  let successCount = 0;
  let errorCount = 0;

  ftlFiles.forEach(file => {
    const ftlPath = path.join(LOCALES_DIR, file);
    const jsonFileName = file.replace('.ftl', '.json');
    const jsonPath = path.join(OUTPUT_DIR, jsonFileName);

    try {
      // Read FTL file
      const ftlContent = fs.readFileSync(ftlPath, 'utf-8');

      // Parse to JSON
      const translations = parseFTL(ftlContent);

      // Count translations
      const translationCount = Object.keys(translations).length;

      // Write JSON file
      fs.writeFileSync(jsonPath, JSON.stringify(translations, null, 2), 'utf-8');

      console.log(`✅ ${file} → ${jsonFileName} (${translationCount} keys)`);
      successCount++;
    } catch (error) {
      console.error(`❌ Error converting ${file}:`, error.message);
      errorCount++;
    }
  });

  console.log(`\n📊 Conversion complete: ${successCount} succeeded, ${errorCount} failed`);
  console.log(`📁 Output directory: ${OUTPUT_DIR}`);
}

// Run conversion
convertTranslations();
