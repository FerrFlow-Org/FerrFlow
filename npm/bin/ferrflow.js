#!/usr/bin/env node
import { spawnSync } from "child_process";
import { existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { createRequire } from "module";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

function getBinaryPath() {
  const platform = process.platform;
  const arch = process.arch;

  const platformMap = {
    "linux-x64": "ferrflow-linux-x64",
    "linux-arm64": "ferrflow-linux-arm64",
    "darwin-x64": "ferrflow-darwin-x64",
    "darwin-arm64": "ferrflow-darwin-arm64",
    "win32-x64": "ferrflow-windows-x64",
  };

  const key = `${platform}-${arch}`;
  const pkgName = platformMap[key];

  if (pkgName) {
    try {
      return require.resolve(`${pkgName}/bin/ferrflow`);
    } catch {
      // optional dep not installed
    }
  }

  // Fallback: local dev build
  const ext = platform === "win32" ? ".exe" : "";
  const devBuild = join(__dirname, "..", "..", "target", "release", `ferrflow${ext}`);
  if (existsSync(devBuild)) return devBuild;

  // Hope it's in PATH
  return platform === "win32" ? "ferrflow.exe" : "ferrflow";
}

const binary = getBinaryPath();
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
process.exit(result.status ?? 1);
