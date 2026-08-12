// Renders the app icon from the demo's hand SVG, cropped to the hand itself.
// Usage: node tools/build-icon.mjs && npx tauri icon icon.png
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const source = join(root, "reference", "pokey_cursor_fast_hold_fx_shake.html");
const html = readFileSync(source, "utf8");

const match = html.match(/<svg id="hero-pointer"[\s\S]*?<\/svg>/);
if (!match) throw new Error("hero-pointer svg not found in reference demo");

// The hand occupies y 0..215 of a 159x1494 viewBox; pad it out to a square canvas.
const svg = match[0]
  .replace(/ data-astro-cid-ge2uvauf/g, "")
  .replace(/width="44"/, 'width="1024"')
  .replace(/height="414"/, 'height="1024"')
  .replace(/viewBox="0 0 159 1494"/, 'viewBox="-38.5 -12 236 236"');

writeFileSync(join(root, "icon.svg"), svg);

await sharp(Buffer.from(svg), { density: 384 })
  .resize(1024, 1024, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
  .png()
  .toFile(join(root, "icon.png"));

console.log("wrote icon.svg + icon.png");
