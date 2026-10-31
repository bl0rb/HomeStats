// Copies the web app (../homestats.html, the single source) to ui/index.html, which Tauri bundles.
// Runs before `tauri dev` and `tauri build` (see src-tauri/tauri.conf.json); ui/ is generated and not committed.
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const desktop = join(dirname(fileURLToPath(import.meta.url)), "..");
mkdirSync(join(desktop, "ui"), { recursive: true });
copyFileSync(join(desktop, "..", "homestats.html"), join(desktop, "ui", "index.html"));
console.log("ui/index.html <- homestats.html");
