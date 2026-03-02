import fs from 'fs';
import path from 'path';

const localesDir = path.join(process.cwd(), 'src/lib/i18n');
const files = ['zh_TW.ts', 'zh_CN.ts', 'en.ts', 'ja.ts', 'ko.ts'];

function extractAllKeysRecursively(filePath) {
    const content = fs.readFileSync(filePath, 'utf-8');
    // Extract everything that looks like a key, including nested ones if possible.
    // A simpler approach: extract everything that matches `word: ` or `'word': `
    const keys = new Set();
    const regex = /^\s*['"]?([a-zA-Z0-9_]+)['"]?\s*:/gm;
    let match;
    while ((match = regex.exec(content)) !== null) {
        keys.add(match[1]);
    }
    return keys;
}

const allKeys = {};
for (const file of files) {
    const filePath = path.join(localesDir, file);
    if (fs.existsSync(filePath)) {
        allKeys[file] = extractAllKeysRecursively(filePath);
        console.log(`Extracted ${allKeys[file].size} keys from ${file}`);
    } else {
        console.error(`File not found: ${file}`);
    }
}

const baseKeys = allKeys['zh_TW.ts'];
if (baseKeys) {
    let hasIssues = false;
    for (const file of files) {
        if (file === 'zh_TW.ts') continue;
        const currentKeys = allKeys[file];
        if (!currentKeys) continue;

        for (const key of baseKeys) {
            if (!currentKeys.has(key)) {
                console.log(`[!] Missing key '${key}' in ${file}`);
                hasIssues = true;
            }
        }
        for (const key of currentKeys) {
            if (!baseKeys.has(key)) {
                console.log(`[!] Extra key '${key}' in ${file}`);
                hasIssues = true;
            }
        }
    }
    if (!hasIssues) {
        console.log("SUCCESS: All i18n keys match perfectly across all languages!");
    } else {
        console.log("WARNING: Mismatches found in i18n files.");
    }
}
