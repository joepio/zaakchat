import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const swPath = path.join(__dirname, '../public/sw.js');

try {
  let content = fs.readFileSync(swPath, 'utf8');
  const timestamp = Math.floor(Date.now() / 1000);
  const newCacheName = `mijnzaken-v${timestamp}`;

  // Regex to find and replace the cache name
  // It looks for: const CACHE_NAME = '...';
  const regex = /const CACHE_NAME = ['"].*?['"];/;

  if (regex.test(content)) {
    const newContent = content.replace(regex, `const CACHE_NAME = '${newCacheName}';`);
    fs.writeFileSync(swPath, newContent, 'utf8');
    console.log(`✅ Service Worker cache bumped to: ${newCacheName}`);
  } else {
    console.error('❌ Could not find CACHE_NAME in sw.js');
    process.exit(1);
  }
} catch (error) {
  console.error('❌ Error updating service worker:', error);
  process.exit(1);
}
