// Syntax check of the inline script in ../homestats.html (CI); fails with the parser error.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const html = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "..", "..", "homestats.html"), "utf8");
const scripts = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]);
if (!scripts.length) throw new Error("homestats.html enthält kein Skript");
scripts.forEach(js => new Function(js));
console.log("homestats.html: " + scripts.length + " Skript(e) syntaktisch in Ordnung");
