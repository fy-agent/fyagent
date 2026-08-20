#!/usr/bin/env node

import {
  constants,
  copyFileSync,
  lstatSync,
  mkdirSync,
  statSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  EXPECTED_INSTALLERS_BY_TARGET,
  assertExactFileSet,
  expectedInstallerNames,
  expectedReleaseAttachmentNames,
  sha256File,
} from "./release-contract.mjs";

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertSafeAssetName(name) {
  assert(
    /^[A-Za-z0-9._-]+$/u.test(name) && path.basename(name) === name,
    `Unsafe Release asset name: ${name}`,
  );
  return name;
}

export function describeReleaseAttachments(directory, version) {
  const names = expectedReleaseAttachmentNames(version);
  const files = assertExactFileSet(directory, names, "Release attachments");
  return Promise.all(
    files.map(async (filePath, index) => ({
      name: assertSafeAssetName(names[index]),
      path: path.resolve(filePath),
      size: statSync(filePath).size,
      sha256: await sha256File(filePath),
    })),
  );
}

export function verifyTargetInstallers(directory, version, targetGroup) {
  const indexes = EXPECTED_INSTALLERS_BY_TARGET[targetGroup];
  assert(indexes, `Unsupported Release target group: ${targetGroup}`);
  const names = expectedInstallerNames(version);
  return assertExactFileSet(
    directory,
    indexes.map((index) => names[index]),
    `${targetGroup} installers`,
  );
}

export async function verifyDownloadedReleaseAttachments({
  sourceDirectory,
  downloadedDirectory,
  version,
}) {
  const expected = await describeReleaseAttachments(sourceDirectory, version);
  const downloaded = await describeReleaseAttachments(
    downloadedDirectory,
    version,
  );
  assert(
    JSON.stringify(
      expected.map(({ name, size, sha256 }) => ({ name, size, sha256 })),
    ) ===
      JSON.stringify(
        downloaded.map(({ name, size, sha256 }) => ({ name, size, sha256 })),
      ),
    "Re-downloaded Release attachments differ from the verified local payload",
  );
  return downloaded;
}

export async function assembleReleaseAttachments({
  subjectsDirectory,
  bundlePath,
  outputDirectory,
  version,
}) {
  mkdirSync(outputDirectory);
  const subjectNames = expectedReleaseAttachmentNames(version).slice(0, -1);
  const subjectPaths = assertExactFileSet(
    subjectsDirectory,
    subjectNames,
    "attestation subjects",
  );
  for (let index = 0; index < subjectPaths.length; index += 1) {
    copyFileSync(
      subjectPaths[index],
      path.join(outputDirectory, subjectNames[index]),
      constants.COPYFILE_EXCL,
    );
  }
  const bundleName = expectedReleaseAttachmentNames(version).at(-1);
  assert(bundleName, "Attestation bundle name is missing");
  const bundleStat = lstatSync(bundlePath);
  assert(
    bundleStat.isFile() && !bundleStat.isSymbolicLink() && bundleStat.size > 0,
    "Attestation bundle must be a non-empty regular file",
  );
  copyFileSync(
    bundlePath,
    path.join(outputDirectory, bundleName),
    constants.COPYFILE_EXCL,
  );
  return describeReleaseAttachments(outputDirectory, version);
}

function parseArguments(argv) {
  const [mode, ...args] = argv;
  if (mode === "list" && args.length === 2) {
    return { mode, directory: path.resolve(args[0]), version: args[1] };
  }
  if (mode === "assemble" && args.length === 4) {
    return {
      mode,
      subjectsDirectory: path.resolve(args[0]),
      bundlePath: path.resolve(args[1]),
      outputDirectory: path.resolve(args[2]),
      version: args[3],
    };
  }
  if (mode === "verify-downloads" && args.length === 3) {
    return {
      mode,
      sourceDirectory: path.resolve(args[0]),
      downloadedDirectory: path.resolve(args[1]),
      version: args[2],
    };
  }
  if (mode === "verify-target" && args.length === 3) {
    return {
      mode,
      directory: path.resolve(args[0]),
      version: args[1],
      targetGroup: args[2],
    };
  }
  throw new Error(
    "Usage: prepare-release-publication.mjs list <attachments> <version> | assemble <subjects> <bundle> <output> <version> | verify-downloads <source> <downloaded> <version> | verify-target <directory> <version> <target-group>",
  );
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : null;
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const input = parseArguments(process.argv.slice(2));
    let attachments;
    if (input.mode === "list") {
      attachments = await describeReleaseAttachments(
        input.directory,
        input.version,
      );
    } else if (input.mode === "assemble") {
      attachments = await assembleReleaseAttachments(input);
    } else if (input.mode === "verify-downloads") {
      attachments = await verifyDownloadedReleaseAttachments(input);
    } else {
      attachments = verifyTargetInstallers(
        input.directory,
        input.version,
        input.targetGroup,
      );
    }
    process.stdout.write(`${JSON.stringify(attachments)}\n`);
  } catch (error) {
    process.stderr.write(
      `${error instanceof Error ? error.message : String(error)}\n`,
    );
    process.exitCode = 1;
  }
}
