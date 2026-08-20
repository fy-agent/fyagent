#!/usr/bin/env node

import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { inflateSync } from "node:zlib";
import {
  ROOT,
  fail,
  isMain,
  repositoryPath,
  run,
  usageBoolean,
  usageList,
  usageValue,
  writeFilesAtomically,
} from "./lib.mjs";

const CANONICAL_VECTOR = "assets/fyagent-y-gate.svg";
const CANONICAL_SOURCE = "assets/fyagent.png";
const CANONICAL_VECTOR_SHA256 =
  "93f0fc710a7046d9a3cd8a713124b046a2a35dc729603096424b01efeef5a43c";
const FYAGENT_Y_PATH = "M7 13 16 6l16 15L48 6l9 7-19 18v27H26V31L7 13Z";
const PNG_SIGNATURE = Buffer.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
]);

const GENERATED_ICONS = Object.freeze([
  "128x128.png",
  "128x128@2x.png",
  "32x32.png",
  "64x64.png",
  "Square107x107Logo.png",
  "Square142x142Logo.png",
  "Square150x150Logo.png",
  "Square284x284Logo.png",
  "Square30x30Logo.png",
  "Square310x310Logo.png",
  "Square44x44Logo.png",
  "Square71x71Logo.png",
  "Square89x89Logo.png",
  "StoreLogo.png",
  "android/mipmap-hdpi/ic_launcher.png",
  "android/mipmap-hdpi/ic_launcher_foreground.png",
  "android/mipmap-hdpi/ic_launcher_round.png",
  "android/mipmap-mdpi/ic_launcher.png",
  "android/mipmap-mdpi/ic_launcher_foreground.png",
  "android/mipmap-mdpi/ic_launcher_round.png",
  "android/mipmap-xhdpi/ic_launcher.png",
  "android/mipmap-xhdpi/ic_launcher_foreground.png",
  "android/mipmap-xhdpi/ic_launcher_round.png",
  "android/mipmap-xxhdpi/ic_launcher.png",
  "android/mipmap-xxhdpi/ic_launcher_foreground.png",
  "android/mipmap-xxhdpi/ic_launcher_round.png",
  "android/mipmap-xxxhdpi/ic_launcher.png",
  "android/mipmap-xxxhdpi/ic_launcher_foreground.png",
  "android/mipmap-xxxhdpi/ic_launcher_round.png",
  "icon.icns",
  "icon.ico",
  "icon.png",
  "ios/AppIcon-20x20@1x.png",
  "ios/AppIcon-20x20@2x-1.png",
  "ios/AppIcon-20x20@2x.png",
  "ios/AppIcon-20x20@3x.png",
  "ios/AppIcon-29x29@1x.png",
  "ios/AppIcon-29x29@2x-1.png",
  "ios/AppIcon-29x29@2x.png",
  "ios/AppIcon-29x29@3x.png",
  "ios/AppIcon-40x40@1x.png",
  "ios/AppIcon-40x40@2x-1.png",
  "ios/AppIcon-40x40@2x.png",
  "ios/AppIcon-40x40@3x.png",
  "ios/AppIcon-512@2x.png",
  "ios/AppIcon-60x60@2x.png",
  "ios/AppIcon-60x60@3x.png",
  "ios/AppIcon-76x76@1x.png",
  "ios/AppIcon-76x76@2x.png",
  "ios/AppIcon-83.5x83.5@2x.png",
]);

const TRAY_ICONS = Object.freeze([
  Object.freeze({
    size: 24,
    destination: "src-tauri/icons/tray/macos/statusTemplate.png",
  }),
  Object.freeze({
    size: 48,
    destination: "src-tauri/icons/tray/macos/statusTemplate@2x.png",
  }),
  Object.freeze({
    size: 72,
    destination: "src-tauri/icons/tray/macos/statusbar_template_3x.png",
  }),
]);

const EXPECTED_ICO_SIZES = Object.freeze([16, 24, 32, 48, 64, 256]);
const ABOUT_ICON = "src/assets/icons/app-icon.png";

function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex");
}

function normalizeRelative(relativePath) {
  return relativePath.split(path.sep).join("/");
}

function listFiles(directory, prefix = "") {
  const result = [];
  for (const entry of fs
    .readdirSync(directory, { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name, "en"))) {
    const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) result.push(...listFiles(absolute, relativePath));
    else if (entry.isFile()) result.push(relativePath);
    else
      throw new Error(`Generated icon is not a regular file: ${relativePath}`);
  }
  return result;
}

function assertExactInventory(actual, expected, label) {
  const normalized = [...actual].map(normalizeRelative).sort();
  const wanted = [...expected].sort();
  if (JSON.stringify(normalized) !== JSON.stringify(wanted)) {
    const missing = wanted.filter((item) => !normalized.includes(item));
    const unexpected = normalized.filter((item) => !wanted.includes(item));
    throw new Error(
      `${label} inventory differs; missing=${missing.join(",") || "none"}; unexpected=${unexpected.join(",") || "none"}`,
    );
  }
}

function assertCanonicalVector() {
  const absolute = repositoryPath(CANONICAL_VECTOR);
  const stat = fs.lstatSync(absolute);
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error("Canonical Y vector must be a regular non-symlink file");
  }
  const bytes = fs.readFileSync(absolute);
  const digest = sha256(bytes);
  if (digest !== CANONICAL_VECTOR_SHA256) {
    throw new Error(
      `Canonical Y vector identity differs: expected ${CANONICAL_VECTOR_SHA256}, received ${digest}`,
    );
  }
  const source = bytes.toString("utf8");
  for (const contract of [
    'viewBox="0 0 1024 1024"',
    'id="fyagent-y-gate"',
    'id="fyagent-y-silhouette"',
    'id="fyagent-gate-outline"',
    `d="${FYAGENT_Y_PATH}"`,
  ]) {
    if (!source.includes(contract)) {
      throw new Error(`Canonical Y vector is missing contract: ${contract}`);
    }
  }
  if (
    /<(?:script|foreignObject)\b|\b(?:href|src)\s*=\s*["']https?:/iu.test(
      source,
    )
  ) {
    throw new Error(
      "Canonical Y vector contains an executable or remote resource",
    );
  }
  return { path: CANONICAL_VECTOR, digest };
}

function pngChunks(buffer, label) {
  if (!buffer.subarray(0, 8).equals(PNG_SIGNATURE)) {
    throw new Error(`Invalid PNG signature: ${label}`);
  }
  const chunks = [];
  let offset = 8;
  while (offset < buffer.length) {
    if (offset + 12 > buffer.length) {
      throw new Error(`Invalid PNG chunk boundary: ${label}`);
    }
    const length = buffer.readUInt32BE(offset);
    const type = buffer.subarray(offset + 4, offset + 8).toString("ascii");
    const payloadStart = offset + 8;
    const payloadEnd = payloadStart + length;
    const chunkEnd = payloadEnd + 4;
    if (!/^[A-Za-z]{4}$/u.test(type) || chunkEnd > buffer.length) {
      throw new Error(`Invalid PNG chunk: ${label}`);
    }
    chunks.push({ type, payload: buffer.subarray(payloadStart, payloadEnd) });
    offset = chunkEnd;
    if (type === "IEND") {
      if (length !== 0 || offset !== buffer.length) {
        throw new Error(`PNG has trailing data: ${label}`);
      }
      break;
    }
  }
  if (chunks[0]?.type !== "IHDR" || chunks.at(-1)?.type !== "IEND") {
    throw new Error(`Incomplete PNG container: ${label}`);
  }
  return chunks;
}

function paeth(left, above, upperLeft) {
  const estimate = left + above - upperLeft;
  const leftDistance = Math.abs(estimate - left);
  const aboveDistance = Math.abs(estimate - above);
  const upperLeftDistance = Math.abs(estimate - upperLeft);
  if (leftDistance <= aboveDistance && leftDistance <= upperLeftDistance)
    return left;
  if (aboveDistance <= upperLeftDistance) return above;
  return upperLeft;
}

export function decodeRgbaPng(buffer, label = "PNG") {
  const chunks = pngChunks(buffer, label);
  const header = chunks[0].payload;
  if (
    header.length !== 13 ||
    header[8] !== 8 ||
    header[9] !== 6 ||
    header[10] !== 0 ||
    header[11] !== 0 ||
    header[12] !== 0
  ) {
    throw new Error(`PNG must be non-interlaced 8-bit RGBA: ${label}`);
  }
  const width = header.readUInt32BE(0);
  const height = header.readUInt32BE(4);
  if (width === 0 || height === 0 || width > 4096 || height > 4096) {
    throw new Error(`PNG dimensions are outside the icon budget: ${label}`);
  }
  const stride = width * 4;
  const compressed = Buffer.concat(
    chunks.filter(({ type }) => type === "IDAT").map(({ payload }) => payload),
  );
  if (compressed.length === 0)
    throw new Error(`PNG has no pixel data: ${label}`);
  const expectedLength = (stride + 1) * height;
  const filtered = inflateSync(compressed, {
    maxOutputLength: expectedLength,
  });
  if (filtered.length !== expectedLength) {
    throw new Error(`PNG pixel payload length differs: ${label}`);
  }
  const pixels = Buffer.alloc(width * height * 4);
  let sourceOffset = 0;
  for (let row = 0; row < height; row += 1) {
    const filter = filtered[sourceOffset++];
    if (filter > 4) throw new Error(`PNG uses an unknown filter: ${label}`);
    const rowOffset = row * stride;
    const previousOffset = rowOffset - stride;
    for (let column = 0; column < stride; column += 1) {
      const raw = filtered[sourceOffset++];
      const left = column >= 4 ? pixels[rowOffset + column - 4] : 0;
      const above = row > 0 ? pixels[previousOffset + column] : 0;
      const upperLeft =
        row > 0 && column >= 4 ? pixels[previousOffset + column - 4] : 0;
      let predictor = 0;
      if (filter === 1) predictor = left;
      else if (filter === 2) predictor = above;
      else if (filter === 3) predictor = Math.floor((left + above) / 2);
      else if (filter === 4) predictor = paeth(left, above, upperLeft);
      pixels[rowOffset + column] = (raw + predictor) & 0xff;
    }
  }
  return { width, height, pixels };
}

export function validateCanonicalPng(buffer, label = CANONICAL_SOURCE) {
  const decoded = decodeRgbaPng(buffer, label);
  if (decoded.width !== 1024 || decoded.height !== 1024) {
    throw new Error(
      `Canonical application icon must be 1024x1024 RGBA: ${label}`,
    );
  }
  const { pixels } = decoded;
  const alphaAt = (x, y) => pixels[(y * decoded.width + x) * 4 + 3];
  for (const [x, y] of [
    [0, 0],
    [1023, 0],
    [0, 1023],
    [1023, 1023],
  ]) {
    if (alphaAt(x, y) !== 0) {
      throw new Error(
        `Canonical application icon corner is not transparent: ${label}`,
      );
    }
  }
  let transparent = 0;
  let partial = 0;
  let opaque = 0;
  let signal = 0;
  for (let offset = 0; offset < pixels.length; offset += 4) {
    const red = pixels[offset];
    const green = pixels[offset + 1];
    const blue = pixels[offset + 2];
    const alpha = pixels[offset + 3];
    if (alpha === 0) transparent += 1;
    else if (alpha === 255) opaque += 1;
    else partial += 1;
    if (
      alpha >= 240 &&
      red <= 40 &&
      green >= 90 &&
      blue >= 170 &&
      blue > green
    ) {
      signal += 1;
    }
  }
  if (transparent < 40_000 || partial === 0 || opaque < 500_000) {
    throw new Error(
      `Canonical application icon alpha contract differs: ${label}`,
    );
  }
  if (signal < 80_000) {
    throw new Error(
      `Canonical application icon has no substantial blue/cyan Y signal: ${label}`,
    );
  }
  return {
    width: decoded.width,
    height: decoded.height,
    transparent,
    partial,
    opaque,
    signal,
    digest: sha256(buffer),
  };
}

export function validateTrayPng(buffer, expectedSize, label) {
  const { width, height, pixels } = decodeRgbaPng(buffer, label);
  if (width !== expectedSize || height !== expectedSize) {
    throw new Error(`Tray template has the wrong size: ${label}`);
  }
  let minimumX = width;
  let minimumY = height;
  let maximumX = -1;
  let maximumY = -1;
  let partial = false;
  let opaque = false;
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (y * width + x) * 4;
      const alpha = pixels[offset + 3];
      if (alpha === 0) continue;
      if (
        pixels[offset] !== 0 ||
        pixels[offset + 1] !== 0 ||
        pixels[offset + 2] !== 0
      ) {
        throw new Error(
          `Tray template contains visible non-black RGB: ${label}`,
        );
      }
      if (alpha === 255) opaque = true;
      else partial = true;
      minimumX = Math.min(minimumX, x);
      minimumY = Math.min(minimumY, y);
      maximumX = Math.max(maximumX, x);
      maximumY = Math.max(maximumY, y);
    }
  }
  if (!partial || !opaque || maximumX < 0) {
    throw new Error(`Tray template has no antialiased black mask: ${label}`);
  }
  const contentWidth = maximumX - minimumX + 1;
  const contentHeight = maximumY - minimumY + 1;
  const scale = expectedSize / 24;
  const centerX = (minimumX + maximumX + 1) / 2;
  const centerY = (minimumY + maximumY + 1) / 2;
  if (
    contentWidth > 19 * scale + 1 ||
    contentHeight > 19 * scale + 1 ||
    contentWidth < 16 * scale ||
    contentHeight < 16 * scale ||
    Math.abs(centerX - expectedSize / 2) > scale ||
    Math.abs(centerY - expectedSize / 2) > scale
  ) {
    throw new Error(
      `Tray template is not centered in an 18pt content box: ${label}`,
    );
  }
  return {
    width,
    height,
    bounds: [minimumX, minimumY, maximumX, maximumY],
    digest: sha256(buffer),
  };
}

export function parseIcoFrames(buffer, label = "icon.ico") {
  if (
    buffer.length < 22 ||
    buffer.readUInt16LE(0) !== 0 ||
    buffer.readUInt16LE(2) !== 1
  ) {
    throw new Error(`Invalid ICO container: ${label}`);
  }
  const count = buffer.readUInt16LE(4);
  const tableEnd = 6 + count * 16;
  if (count === 0 || tableEnd > buffer.length) {
    throw new Error(`Invalid ICO table: ${label}`);
  }
  const ranges = [];
  const frames = [];
  for (let index = 0; index < count; index += 1) {
    const entry = 6 + index * 16;
    const width = buffer[entry] || 256;
    const height = buffer[entry + 1] || 256;
    const length = buffer.readUInt32LE(entry + 8);
    const offset = buffer.readUInt32LE(entry + 12);
    const end = offset + length;
    if (
      width !== height ||
      offset < tableEnd ||
      length === 0 ||
      end > buffer.length
    ) {
      throw new Error(`Invalid ICO frame: ${label}`);
    }
    if (!buffer.subarray(offset, offset + 8).equals(PNG_SIGNATURE)) {
      throw new Error(`ICO frame is not PNG-backed: ${label}`);
    }
    decodeRgbaPng(buffer.subarray(offset, end), `${label}#${width}`);
    frames.push({
      width,
      height,
      colorCount: buffer[entry + 2],
      bitCount: buffer.readUInt16LE(entry + 6),
      digest: sha256(buffer.subarray(offset, end)),
    });
    ranges.push({ offset, end });
  }
  ranges.sort((left, right) => left.offset - right.offset);
  let cursor = tableEnd;
  for (const range of ranges) {
    if (range.offset !== cursor)
      throw new Error(`ICO frame ranges differ: ${label}`);
    cursor = range.end;
  }
  if (cursor !== buffer.length)
    throw new Error(`ICO has trailing data: ${label}`);
  const sizes = frames
    .map(({ width }) => width)
    .sort((left, right) => left - right);
  if (JSON.stringify(sizes) !== JSON.stringify(EXPECTED_ICO_SIZES)) {
    throw new Error(`ICO frame sizes differ: ${label}`);
  }
  if (
    frames.some(
      ({ colorCount, bitCount }) => colorCount !== 0 || bitCount !== 32,
    )
  ) {
    throw new Error(`ICO frame color contract differs: ${label}`);
  }
  return frames;
}

function canonicalizeIcns(buffer, label = "icon.icns") {
  if (
    buffer.length < 8 ||
    buffer.subarray(0, 4).toString("ascii") !== "icns" ||
    buffer.readUInt32BE(4) !== buffer.length
  ) {
    throw new Error(`Invalid ICNS container: ${label}`);
  }
  const chunks = [];
  let offset = 8;
  while (offset < buffer.length) {
    if (offset + 8 > buffer.length)
      throw new Error(`Invalid ICNS chunk: ${label}`);
    const length = buffer.readUInt32BE(offset + 4);
    if (length < 8 || offset + length > buffer.length) {
      throw new Error(`Invalid ICNS chunk length: ${label}`);
    }
    const chunk = buffer.subarray(offset, offset + length);
    const payload = chunk.subarray(8);
    if (payload.subarray(0, 8).equals(PNG_SIGNATURE)) {
      decodeRgbaPng(
        payload,
        `${label}:${chunk.subarray(0, 4).toString("ascii")}`,
      );
    }
    chunks.push(Buffer.from(chunk));
    offset += length;
  }
  if (offset !== buffer.length || chunks.length === 0) {
    throw new Error(`Incomplete ICNS container: ${label}`);
  }
  chunks.sort((left, right) => {
    const typeOrder = left.subarray(0, 4).compare(right.subarray(0, 4));
    return typeOrder === 0 ? left.compare(right) : typeOrder;
  });
  const output = Buffer.alloc(
    8 + chunks.reduce((sum, chunk) => sum + chunk.length, 0),
  );
  output.write("icns", 0, "ascii");
  output.writeUInt32BE(output.length, 4);
  let outputOffset = 8;
  for (const chunk of chunks) {
    chunk.copy(output, outputOffset);
    outputOffset += chunk.length;
  }
  return output;
}

function createTraySvg() {
  const scale = 18 / 52;
  const translateX = (24 - 50 * scale) / 2;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
  <defs>
    <mask id="gate-cut" maskUnits="userSpaceOnUse" x="0" y="0" width="64" height="64">
      <rect width="64" height="64" fill="#fff"/>
      <rect x="24" y="24" width="16" height="16" rx="3.5" fill="#000"/>
    </mask>
  </defs>
  <g transform="translate(${translateX} 3) scale(${scale}) translate(-7 -6)">
    <path d="${FYAGENT_Y_PATH}" fill="#000" mask="url(#gate-cut)"/>
    <rect x="25" y="25" width="14" height="14" rx="3" fill="none" stroke="#000" stroke-width="3"/>
  </g>
</svg>
`;
}

function withTemporaryDirectory(action) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), "fyagent-icons-"));
  try {
    return action(directory);
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

function runTauriIcon(input, output, pngSizes = []) {
  fs.mkdirSync(output, { recursive: true });
  const args = ["tauri", "icon", input, "--output", output];
  for (const size of pngSizes) args.push("--png", String(size));
  run("pnpm", args);
}

function renderCanonicalSource(temporaryDirectory) {
  const output = path.join(temporaryDirectory, "canonical");
  runTauriIcon(CANONICAL_VECTOR, output, [1024]);
  const generatedPath = path.join(output, "1024x1024.png");
  const bytes = fs.readFileSync(generatedPath);
  validateCanonicalPng(bytes, `${CANONICAL_VECTOR} render`);
  return bytes;
}

function renderBundleIcons(temporaryDirectory) {
  const output = path.join(temporaryDirectory, "bundle");
  // The checked-in 1024 RGBA PNG is deliberately the input here. This keeps
  // the package source path authoritative instead of bypassing it with SVG.
  runTauriIcon(CANONICAL_SOURCE, output);
  assertExactInventory(listFiles(output), GENERATED_ICONS, "Tauri icon output");
  const files = new Map();
  for (const relativePath of GENERATED_ICONS) {
    let bytes = fs.readFileSync(path.join(output, ...relativePath.split("/")));
    if (relativePath === "icon.icns") {
      // Tauri's ICNS payload order is nondeterministic. Sorting untouched
      // chunks makes repeated generation byte-stable while preserving pixels.
      bytes = canonicalizeIcns(bytes);
    }
    if (relativePath === "icon.ico") parseIcoFrames(bytes);
    else if (relativePath.endsWith(".png")) decodeRgbaPng(bytes, relativePath);
    files.set(relativePath, bytes);
  }
  return files;
}

function renderTrayIcons(temporaryDirectory) {
  const source = path.join(temporaryDirectory, "fyagent-y-tray.svg");
  fs.writeFileSync(source, createTraySvg(), { encoding: "utf8", flag: "wx" });
  const output = path.join(temporaryDirectory, "tray");
  runTauriIcon(
    source,
    output,
    TRAY_ICONS.map(({ size }) => size),
  );
  const expected = TRAY_ICONS.map(({ size }) => `${size}x${size}.png`);
  assertExactInventory(listFiles(output), expected, "Tray icon output");
  return new Map(
    TRAY_ICONS.map(({ size, destination }) => {
      const bytes = fs.readFileSync(path.join(output, `${size}x${size}.png`));
      validateTrayPng(bytes, size, destination);
      return [destination, bytes];
    }),
  );
}

function assertEqualBytes(actual, expected, label) {
  if (!actual.equals(expected)) {
    throw new Error(
      `${label} differs from canonical generation: expected ${sha256(expected)}, received ${sha256(actual)}`,
    );
  }
}

function assertConfiguredConsumers() {
  const tauri = JSON.parse(
    fs.readFileSync(repositoryPath("src-tauri/tauri.conf.json"), "utf8"),
  );
  const configured = tauri.bundle?.icon;
  const expected = [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico",
  ];
  if (JSON.stringify(configured) !== JSON.stringify(expected)) {
    throw new Error("Tauri application icon consumer list differs");
  }
  const windows = JSON.parse(
    fs.readFileSync(
      repositoryPath("src-tauri/tauri.windows.conf.json"),
      "utf8",
    ),
  );
  if (windows.bundle?.windows?.nsis?.installerIcon !== "icons/icon.ico") {
    throw new Error("Windows NSIS installer icon is not canonical icon.ico");
  }
  const rust = fs.readFileSync(repositoryPath("src-tauri/src/lib.rs"), "utf8");
  if (
    !rust.includes(
      'include_bytes!("../icons/tray/macos/statusbar_template_3x.png")',
    )
  ) {
    throw new Error(
      "macOS tray runtime is not embedding the canonical 3x template",
    );
  }
}

function validateStoredAssets(canonicalBytes, generated, trays) {
  const storedCanonical = fs.readFileSync(repositoryPath(CANONICAL_SOURCE));
  validateCanonicalPng(storedCanonical);
  assertEqualBytes(storedCanonical, canonicalBytes, CANONICAL_SOURCE);

  for (const [relativePath, expected] of generated) {
    const destination = `src-tauri/icons/${relativePath}`;
    const actual = fs.readFileSync(repositoryPath(destination));
    assertEqualBytes(actual, expected, destination);
  }

  const generatedAbout = generated.get("32x32.png");
  if (!generatedAbout)
    throw new Error("Tauri output has no 32x32 About source");
  assertEqualBytes(
    fs.readFileSync(repositoryPath(ABOUT_ICON)),
    generatedAbout,
    ABOUT_ICON,
  );

  for (const [destination, expected] of trays) {
    const actual = fs.readFileSync(repositoryPath(destination));
    validateTrayPng(
      actual,
      TRAY_ICONS.find((item) => item.destination === destination).size,
      destination,
    );
    assertEqualBytes(actual, expected, destination);
  }

  assertConfiguredConsumers();
  return {
    source: {
      path: CANONICAL_SOURCE,
      digest: sha256(storedCanonical),
      width: 1024,
      height: 1024,
      mode: "RGBA",
    },
    generated: generated.size,
    about: ABOUT_ICON,
    trays: trays.size,
    icoFrames: parseIcoFrames(generated.get("icon.ico")).map(
      ({ width }) => width,
    ),
  };
}

function validateSourceArgument() {
  const source = normalizeRelative(usageValue("source") ?? CANONICAL_SOURCE);
  if (source !== CANONICAL_SOURCE) {
    throw new Error(
      `Application icons have one canonical package source; --source must be ${CANONICAL_SOURCE}`,
    );
  }
  return source;
}

export function applyApplicationBrandAssets() {
  validateSourceArgument();
  assertCanonicalVector();
  return withTemporaryDirectory((temporaryDirectory) => {
    const canonicalBytes = renderCanonicalSource(temporaryDirectory);
    const originalCanonical = fs.readFileSync(repositoryPath(CANONICAL_SOURCE));
    writeFilesAtomically([[CANONICAL_SOURCE, canonicalBytes]]);

    // Generate the entire Tauri inventory only after the canonical repository
    // path contains the reviewed 1024 RGBA bytes.
    let generated;
    let trays;
    let generationError;
    try {
      generated = renderBundleIcons(temporaryDirectory);
      trays = renderTrayIcons(temporaryDirectory);
    } catch (error) {
      generationError = error;
    }

    let restoreError;
    try {
      // Restore the preimage before the final atomic multi-file write. A
      // failed Tauri/tray generation therefore cannot leave only the source
      // changed, while the successful Tauri invocation still consumed the
      // required repository path.
      writeFilesAtomically([[CANONICAL_SOURCE, originalCanonical]]);
    } catch (error) {
      restoreError = error;
    }
    if (generationError !== undefined || restoreError !== undefined) {
      const failures = [generationError, restoreError].filter(
        (error) => error !== undefined,
      );
      if (failures.length === 1) throw failures[0];
      throw new AggregateError(
        failures,
        "Application icon generation and source recovery both failed",
      );
    }

    const changes = [[CANONICAL_SOURCE, canonicalBytes]];
    for (const [relativePath, bytes] of generated) {
      changes.push([`src-tauri/icons/${relativePath}`, bytes]);
    }
    changes.push([ABOUT_ICON, generated.get("32x32.png")]);
    changes.push(...trays);
    writeFilesAtomically(changes);

    const result = validateStoredAssets(canonicalBytes, generated, trays);
    console.log(JSON.stringify({ status: "applied", ...result }, null, 2));
    return result;
  });
}

export function checkApplicationBrandAssets() {
  validateSourceArgument();
  const vector = assertCanonicalVector();
  return withTemporaryDirectory((temporaryDirectory) => {
    const canonicalBytes = renderCanonicalSource(temporaryDirectory);
    const storedCanonical = fs.readFileSync(repositoryPath(CANONICAL_SOURCE));
    assertEqualBytes(storedCanonical, canonicalBytes, CANONICAL_SOURCE);
    const generated = renderBundleIcons(temporaryDirectory);
    const trays = renderTrayIcons(temporaryDirectory);
    const result = validateStoredAssets(canonicalBytes, generated, trays);
    console.log(
      JSON.stringify({ status: "verified", vector, ...result }, null, 2),
    );
    return result;
  });
}

function test(watch) {
  const filters = usageList("filters");
  for (const filter of filters) {
    if (filter.startsWith("-")) {
      throw new Error(
        "test filters accept file or test-name values only; Vitest options are forbidden",
      );
    }
  }
  run("pnpm", [watch ? "test:unit:watch" : "test:unit", ...filters]);
}

function visualUpdate() {
  const evidence = usageValue("evidence");
  if (!evidence) throw new Error("A reviewed evidence JSON file is required");
  repositoryPath(evidence);
  run("pnpm", ["test:desktop:visual:update", evidence]);
}

function assetsIcons() {
  validateSourceArgument();
  if (!usageBoolean("apply")) {
    console.log(
      JSON.stringify(
        {
          status: "preview",
          source: CANONICAL_SOURCE,
          vector: CANONICAL_VECTOR,
          steps: [
            "render reviewed vector to canonical 1024 RGBA PNG",
            "generate the complete Tauri icon inventory from canonical PNG",
            "synchronize About 32px and macOS 1x/2x/3x tray templates",
          ],
        },
        null,
        2,
      ),
    );
    return;
  }
  applyApplicationBrandAssets();
}

function main() {
  switch (process.argv[2]) {
    case "test-unit":
      test(false);
      break;
    case "test-watch":
      test(true);
      break;
    case "visual-update":
      visualUpdate();
      break;
    case "assets-icons":
      assetsIcons();
      break;
    case "assets-icons-check":
      checkApplicationBrandAssets();
      break;
    default:
      throw new Error(
        `Unknown frontend task command: ${process.argv[2] ?? ""}`,
      );
  }
}

if (isMain(import.meta.url)) {
  try {
    main();
  } catch (error) {
    fail(error);
  }
}
