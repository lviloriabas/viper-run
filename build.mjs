#!/usr/bin/env node
/* Tiny zero-dependency build: inline CSS, JS and fonts into one HTML file. */
import { readFileSync, writeFileSync, mkdirSync, copyFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url));
const read = (p) => readFileSync(resolve(root, p), "utf8");

let html = read("src/index.html");
let css = read("src/style.css");
const game = read("src/game.js");
const app = read("src/app.js");
const cockpit = read("src/cockpit.js");
const airframe = read("src/airframe.js");

// Font files are referenced relatively in source; bake them into the page.
css = css.replace(/url\((fonts\/[^)]+)\)/g, (_, rel) => {
  const data = readFileSync(resolve(root, "src", rel));
  return `url(data:font/ttf;base64,${data.toString("base64")})`;
});

html = html.replace('<link rel="stylesheet" href="style.css">', `<style>\n${css}\n</style>`);
html = html.replace(
  '<script src="airframe.js"></script>\n<script src="game.js"></script>\n<script src="cockpit.js"></script>\n<script src="app.js"></script>',
  `<script>\n${airframe}\n</script>\n<script>\n${game}\n</script>\n<script>\n${cockpit}\n</script>\n<script>\n${app}\n</script>`
);

mkdirSync(resolve(root, "dist"), { recursive: true });
const out = resolve(root, "dist/viper-run.html");
writeFileSync(out, html, "utf8");
copyFileSync(out, resolve(root, "dist/index.html"));

const bytes = Buffer.byteLength(html, "utf8");
console.log(`built dist/viper-run.html  (${(bytes / 1024).toFixed(1)} KiB, self-contained)`);
