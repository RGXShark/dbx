import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const root = fileURLToPath(new URL("../", import.meta.url));
const config = resolve(root, "config/enterprise-ai.json");
if (!existsSync(config)) {
  console.error("Create config/enterprise-ai.json from config/enterprise-ai.example.json before packaging.");
  process.exit(1);
}
// Invoke pnpm's JS entry point directly on Windows; never interpolate CLI arguments into a shell.
const pnpm = process.env.npm_execpath;
if (!pnpm) throw new Error("Run this script through pnpm build:enterprise");
const result = spawnSync(process.execPath, [pnpm, "tauri", "build", "--config", JSON.stringify({ bundle: { createUpdaterArtifacts: false } }), ...process.argv.slice(2)], {
  cwd: root,
  env: { ...process.env, DBX_ENTERPRISE_AI_CONFIG: config },
  stdio: "inherit",
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
