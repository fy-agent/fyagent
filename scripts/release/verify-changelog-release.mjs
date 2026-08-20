#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  assertChangelogMatchesVersion,
  readCargoWorkspaceVersion,
} from "./release-contract.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

const version = readCargoWorkspaceVersion(
  readFileSync(join(ROOT, "src-tauri", "Cargo.toml"), "utf8"),
);
assertChangelogMatchesVersion(
  readFileSync(join(ROOT, "CHANGELOG.md"), "utf8"),
  version,
);
