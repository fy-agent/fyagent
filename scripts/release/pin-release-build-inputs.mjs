#!/usr/bin/env node

import {
  constants,
  copyFileSync,
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  EXPECTED_INSTALLERS_BY_TARGET,
  EXPECTED_TARGETS,
  PRODUCT_NAME,
  assertExactDirectorySet,
  assertExactFileSet,
  expectedInstallerNames,
  sha256File,
} from "./release-contract.mjs";

export const TRUSTED_BUILD_INPUTS_SCHEMA = "fyagent-release-build-inputs/v1";
export const TRUSTED_BUILD_INPUTS_MANIFEST = "trusted-build-inputs.json";

const RAW_TARGETS = Object.freeze(["windows-x64", "windows-arm64"]);
const INSTALLER_TARGETS = Object.freeze(["macos-universal"]);
const METADATA_TARGETS = Object.freeze(
  EXPECTED_TARGETS.map(({ targetGroup }) => targetGroup),
);
const ARTIFACT_NAMES = Object.freeze([
  ...RAW_TARGETS.map((target) => `raw-${target}`),
  ...INSTALLER_TARGETS.map((target) => `installers-${target}`),
  ...METADATA_TARGETS.map((target) => `metadata-${target}`),
]);
const SOURCE_SHA_PATTERN = /^[0-9a-f]{40}$/u;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function expectedFilesByArtifact(version) {
  const installerNames = expectedInstallerNames(version);
  return Object.freeze(
    Object.fromEntries([
      ...RAW_TARGETS.map((target) => [
        `raw-${target}`,
        EXPECTED_INSTALLERS_BY_TARGET[target].map(
          (index) => installerNames[index],
        ),
      ]),
      ...INSTALLER_TARGETS.map((target) => [
        `installers-${target}`,
        EXPECTED_INSTALLERS_BY_TARGET[target].map(
          (index) => installerNames[index],
        ),
      ]),
      ...METADATA_TARGETS.map((target) => [
        `metadata-${target}`,
        [`${target}.json`],
      ]),
    ]),
  );
}

function assertIdentity(version, sourceSha) {
  expectedInstallerNames(version);
  assert(
    SOURCE_SHA_PATTERN.test(sourceSha),
    "Pinned release source SHA must be a lowercase full 40-character Git commit SHA",
  );
}

function assertRegularRootEntry(root, entry) {
  const entryPath = path.join(root, entry.name);
  assert(
    !lstatSync(entryPath).isSymbolicLink(),
    `Symbolic links are forbidden in trusted build inputs: ${entry.name}`,
  );
  assert(
    entry.isDirectory() ||
      (entry.isFile() && entry.name === TRUSTED_BUILD_INPUTS_MANIFEST),
    `Unexpected trusted build input entry: ${entry.name}`,
  );
}

function assertTrustedRootSet(root) {
  const entries = readdirSync(root, { withFileTypes: true });
  for (const entry of entries) assertRegularRootEntry(root, entry);
  const actual = entries.map(({ name }) => name).sort();
  const expected = [...ARTIFACT_NAMES, TRUSTED_BUILD_INPUTS_MANIFEST].sort();
  assert(
    actual.length === expected.length &&
      actual.every((name, index) => name === expected[index]),
    `Trusted build inputs must contain exactly ${expected.join(", ")}; received ${actual.join(", ")}`,
  );
}

async function buildManifest(root, version, sourceSha) {
  const expectedFiles = expectedFilesByArtifact(version);
  const artifacts = [];
  for (const artifact of ARTIFACT_NAMES) {
    const artifactRoot = path.join(root, artifact);
    const filePaths = assertExactFileSet(
      artifactRoot,
      expectedFiles[artifact],
      `${artifact} pinned release input`,
    );
    const files = [];
    for (const filePath of filePaths) {
      const size = statSync(filePath).size;
      assert(size > 0, `Pinned release input must not be empty: ${filePath}`);
      files.push(
        Object.freeze({
          path: `${artifact}/${path.basename(filePath)}`,
          size,
          sha256: await sha256File(filePath),
        }),
      );
    }
    artifacts.push(Object.freeze({ name: artifact, files }));
  }
  return Object.freeze({
    schema: TRUSTED_BUILD_INPUTS_SCHEMA,
    product: PRODUCT_NAME,
    version,
    sourceSha,
    artifacts,
  });
}

function parseManifest(manifestPath) {
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (error) {
    throw new Error(
      `Unable to parse trusted build inputs manifest: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  return parsed;
}

export async function createTrustedBuildInputs({
  inputRoot,
  outputRoot,
  version,
  sourceSha,
}) {
  assertIdentity(version, sourceSha);
  assertExactDirectorySet(
    inputRoot,
    ARTIFACT_NAMES,
    "release build input download root",
  );
  mkdirSync(outputRoot);
  const expectedFiles = expectedFilesByArtifact(version);
  for (const artifact of ARTIFACT_NAMES) {
    const inputArtifact = path.join(inputRoot, artifact);
    const outputArtifact = path.join(outputRoot, artifact);
    const filePaths = assertExactFileSet(
      inputArtifact,
      expectedFiles[artifact],
      `${artifact} release build input`,
    );
    mkdirSync(outputArtifact);
    for (const filePath of filePaths) {
      copyFileSync(
        filePath,
        path.join(outputArtifact, path.basename(filePath)),
        constants.COPYFILE_EXCL,
      );
    }
  }
  const manifest = await buildManifest(outputRoot, version, sourceSha);
  writeFileSync(
    path.join(outputRoot, TRUSTED_BUILD_INPUTS_MANIFEST),
    `${JSON.stringify(manifest, null, 2)}\n`,
    { flag: "wx" },
  );
  await verifyTrustedBuildInputs({ root: outputRoot, version, sourceSha });
  return manifest;
}

export async function verifyTrustedBuildInputs({ root, version, sourceSha }) {
  assertIdentity(version, sourceSha);
  assertTrustedRootSet(root);
  const actual = parseManifest(path.join(root, TRUSTED_BUILD_INPUTS_MANIFEST));
  const expected = await buildManifest(root, version, sourceSha);
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    "Trusted build inputs manifest does not exactly bind the expected files, sizes, digests, version, and source SHA",
  );
  return expected;
}

function parseArguments(argv) {
  const [mode, ...args] = argv;
  if (mode === "create" && args.length === 4) {
    return {
      mode,
      options: {
        inputRoot: path.resolve(args[0]),
        outputRoot: path.resolve(args[1]),
        version: args[2],
        sourceSha: args[3],
      },
    };
  }
  if (mode === "verify" && args.length === 3) {
    return {
      mode,
      options: {
        root: path.resolve(args[0]),
        version: args[1],
        sourceSha: args[2],
      },
    };
  }
  throw new Error(
    "Usage: pin-release-build-inputs.mjs create <download-root> <output-root> <version> <source-sha> | verify <trusted-root> <version> <source-sha>",
  );
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : null;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const { mode, options } = parseArguments(process.argv.slice(2));
    const manifest =
      mode === "create"
        ? await createTrustedBuildInputs(options)
        : await verifyTrustedBuildInputs(options);
    process.stdout.write(
      `Trusted release build inputs ${mode === "create" ? "created" : "verified"} (${manifest.artifacts.length} artifacts)\n`,
    );
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
