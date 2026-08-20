import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  parseIcoFrames,
  validateCanonicalPng,
  validateTrayPng,
} from "../scripts/tasks/frontend.mjs";

const ROOT = path.resolve(__dirname, "..");
const CANONICAL_VECTOR = path.join(ROOT, "assets", "fyagent-y-gate.svg");
const CANONICAL_PNG = path.join(ROOT, "assets", "fyagent.png");
const ICONS = path.join(ROOT, "src-tauri", "icons");
const ABOUT = path.join(ROOT, "src", "assets", "icons", "app-icon.png");
const OLD_APPLICATION_ICON_SHA256 =
  "352e3695331eb12c44946be46512489a595d11031c9bcb312deb1141b9bf24be";
const EXPECTED_VECTOR_SHA256 =
  "93f0fc710a7046d9a3cd8a713124b046a2a35dc729603096424b01efeef5a43c";
const EXPECTED_PNG_SHA256 =
  "9e2ceb57c5614a15e73c1812b2013b2b53b34ebbd9289e6c39d5c0f453f77a0f";

function sha256(value: Buffer): string {
  return createHash("sha256").update(value).digest("hex");
}

describe("FyAgent application brand assets", () => {
  it("keeps the reviewed high-resolution Y geometry as an auditable source", () => {
    const bytes = fs.readFileSync(CANONICAL_VECTOR);
    const source = bytes.toString("utf8");

    expect(sha256(bytes)).toBe(EXPECTED_VECTOR_SHA256);
    expect(source).toContain('viewBox="0 0 1024 1024"');
    expect(source).toContain('id="fyagent-y-gate"');
    expect(source).toContain('id="fyagent-y-silhouette"');
    expect(source).toContain('id="fyagent-gate-outline"');
    expect(source).toContain(
      'd="M7 13 16 6l16 15L48 6l9 7-19 18v27H26V31L7 13Z"',
    );
    expect(source).not.toMatch(
      /<(?:script|foreignObject)\b|\b(?:href|src)\s*=\s*["']https?:/iu,
    );
  });

  it("replaces the old package source with the 1024 RGBA transparent Y icon", () => {
    const bytes = fs.readFileSync(CANONICAL_PNG);
    const result = validateCanonicalPng(bytes, "assets/fyagent.png");

    expect(result.digest).toBe(EXPECTED_PNG_SHA256);
    expect(result.digest).not.toBe(OLD_APPLICATION_ICON_SHA256);
    expect(result).toMatchObject({ width: 1024, height: 1024 });
    expect(result.transparent).toBeGreaterThan(40_000);
    expect(result.partial).toBeGreaterThan(0);
    expect(result.opaque).toBeGreaterThan(500_000);
    expect(result.signal).toBeGreaterThan(80_000);
  });

  it("keeps About byte-identical to generated 32px and validates ICO frames", () => {
    expect(fs.readFileSync(ABOUT)).toEqual(
      fs.readFileSync(path.join(ICONS, "32x32.png")),
    );
    const frames = parseIcoFrames(
      fs.readFileSync(path.join(ICONS, "icon.ico")),
    );

    expect(frames.map(({ width }) => width).sort((a, b) => a - b)).toEqual([
      16, 24, 32, 48, 64, 256,
    ]);
    expect(frames.every(({ colorCount }) => colorCount === 0)).toBe(true);
    expect(frames.every(({ bitCount }) => bitCount === 32)).toBe(true);
  });

  it.each([
    ["statusTemplate.png", 24],
    ["statusTemplate@2x.png", 48],
    ["statusbar_template_3x.png", 72],
  ] as const)(
    "keeps %s as a centered antialiased black RGBA template",
    (filename, size) => {
      const result = validateTrayPng(
        fs.readFileSync(path.join(ICONS, "tray", "macos", filename)),
        size,
        filename,
      );

      expect(result).toMatchObject({ width: size, height: size });
    },
  );

  it("preserves canonical Tauri, NSIS, and runtime consumer paths", () => {
    const tauri = JSON.parse(
      fs.readFileSync(path.join(ROOT, "src-tauri", "tauri.conf.json"), "utf8"),
    ) as { bundle: { icon: string[] } };
    const windows = JSON.parse(
      fs.readFileSync(
        path.join(ROOT, "src-tauri", "tauri.windows.conf.json"),
        "utf8",
      ),
    ) as { bundle: { windows: { nsis: { installerIcon: string } } } };
    const rust = fs.readFileSync(
      path.join(ROOT, "src-tauri", "src", "lib.rs"),
      "utf8",
    );

    expect(tauri.bundle.icon).toEqual([
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico",
    ]);
    expect(windows.bundle.windows.nsis.installerIcon).toBe("icons/icon.ico");
    expect(rust).toContain(
      'include_bytes!("../icons/tray/macos/statusbar_template_3x.png")',
    );
  });
});
