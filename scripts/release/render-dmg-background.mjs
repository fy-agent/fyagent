#!/usr/bin/env node

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { crc32, deflateSync } from "node:zlib";
import { isMain } from "../tasks/lib.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

export const DMG_BACKGROUND_PATH = "src-tauri/icons/dmg-background.png";
export const DMG_WINDOW_WIDTH_PT = 660;
export const DMG_WINDOW_HEIGHT_PT = 400;
export const DMG_BACKGROUND_SCALE = 2;
export const DMG_BACKGROUND_WIDTH = DMG_WINDOW_WIDTH_PT * DMG_BACKGROUND_SCALE;
export const DMG_BACKGROUND_HEIGHT = DMG_WINDOW_HEIGHT_PT * DMG_BACKGROUND_SCALE;
export const DMG_BACKGROUND_PIXELS_PER_METER = 5669;
export const DMG_ICON_SIZE_PT = 128;
export const DMG_APP_XY = Object.freeze([180, 188]);
export const DMG_APPLICATIONS_XY = Object.freeze([480, 188]);

const FY_BG = Object.freeze([0x32, 0x4d, 0x69]);
const FY_BG_MID = Object.freeze([0x56, 0x74, 0x95]);
const FY_BG_AIR = Object.freeze([0x7b, 0x99, 0xb8]);
const FY_ACCENT = Object.freeze([0x9d, 0xdc, 0xff]);
const FY_ACCENT_HOVER = Object.freeze([0xc4, 0xeb, 0xff]);

function mix(left, right, t) {
  return left + (right - left) * t;
}

function mixRgb(left, right, t) {
  return [
    mix(left[0], right[0], t),
    mix(left[1], right[1], t),
    mix(left[2], right[2], t),
  ];
}

function clamp01(value) {
  if (value < 0) return 0;
  if (value > 1) return 1;
  return value;
}

function over(dst, src, alpha) {
  const a = clamp01(alpha);
  dst[0] = mix(dst[0], src[0], a);
  dst[1] = mix(dst[1], src[1], a);
  dst[2] = mix(dst[2], src[2], a);
}

function verticalBackdrop(t) {
  if (t < 0.5) return mixRgb(FY_BG, FY_BG_MID, t * 2);
  return mixRgb(FY_BG_MID, FY_BG_AIR, (t - 0.5) * 2);
}

function filledCircle(px, py, cx, cy, radius) {
  const dx = px - cx;
  const dy = py - cy;
  return dx * dx + dy * dy <= radius * radius;
}

function pointInTriangle(px, py, ax, ay, bx, by, cx, cy) {
  const v0x = cx - ax;
  const v0y = cy - ay;
  const v1x = bx - ax;
  const v1y = by - ay;
  const v2x = px - ax;
  const v2y = py - ay;
  const dot00 = v0x * v0x + v0y * v0y;
  const dot01 = v0x * v1x + v0y * v1y;
  const dot02 = v0x * v2x + v0y * v2y;
  const dot11 = v1x * v1x + v1y * v1y;
  const dot12 = v1x * v2x + v1y * v2y;
  const denom = dot00 * dot11 - dot01 * dot01;
  if (denom === 0) return false;
  const u = (dot11 * dot02 - dot01 * dot12) / denom;
  const v = (dot00 * dot12 - dot01 * dot02) / denom;
  return u >= 0 && v >= 0 && u + v <= 1;
}

function inRoundedRect(px, py, left, top, right, bottom, radius) {
  if (px < left || px > right || py < top || py > bottom) return false;
  const innerLeft = left + radius;
  const innerRight = right - radius;
  const innerTop = top + radius;
  const innerBottom = bottom - radius;
  if (px >= innerLeft && px <= innerRight) return true;
  if (py >= innerTop && py <= innerBottom) return true;
  const cx = px < innerLeft ? innerLeft : innerRight;
  const cy = py < innerTop ? innerTop : innerBottom;
  return filledCircle(px, py, cx, cy, radius);
}

export function renderDmgBackgroundRgb() {
  const width = DMG_BACKGROUND_WIDTH;
  const height = DMG_BACKGROUND_HEIGHT;
  const pixels = Buffer.alloc(width * height * 3);
  const appCx = DMG_APP_XY[0] * DMG_BACKGROUND_SCALE;
  const appsCx = DMG_APPLICATIONS_XY[0] * DMG_BACKGROUND_SCALE;
  const wellCy = DMG_APP_XY[1] * DMG_BACKGROUND_SCALE;
  const wellRadius = 78;
  const glowCx = width / 2;
  const arrowY = wellCy;
  const shaftLeft = 508;
  const shaftRight = 772;
  const shaftHalf = 7;
  const headLeft = 748;
  const headRight = 812;
  const headHalf = 22;

  for (let y = 0; y < height; y += 1) {
    const backdrop = verticalBackdrop(height === 1 ? 0 : y / (height - 1));
    for (let x = 0; x < width; x += 1) {
      const color = backdrop.slice();
      const glowDist = Math.hypot(x - glowCx, y - 36);
      over(color, FY_ACCENT, (1 - clamp01(glowDist / 430)) ** 2 * 0.16);

      for (const cx of [appCx, appsCx]) {
        const dx = x - cx;
        const dy = y - wellCy;
        const dist = Math.hypot(dx, dy);
        if (dist <= wellRadius) {
          over(color, [246, 251, 255], 0.06);
          over(color, FY_ACCENT, (1 - dist / wellRadius) * 0.07);
        } else if (dist <= wellRadius + 2) {
          over(color, FY_ACCENT, 0.18);
        }
      }

      if (
        inRoundedRect(
          x,
          y,
          shaftLeft,
          arrowY - shaftHalf,
          shaftRight,
          arrowY + shaftHalf,
          6,
        ) ||
        pointInTriangle(
          x,
          y,
          headLeft,
          arrowY - headHalf,
          headLeft,
          arrowY + headHalf,
          headRight,
          arrowY,
        )
      ) {
        over(color, FY_ACCENT_HOVER, 0.92);
      }

      const offset = (y * width + x) * 3;
      pixels[offset] = Math.round(color[0]);
      pixels[offset + 1] = Math.round(color[1]);
      pixels[offset + 2] = Math.round(color[2]);
    }
  }

  return { width, height, pixels };
}

function pngChunk(type, data) {
  const typeBuffer = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuffer, data])) >>> 0);
  return Buffer.concat([length, typeBuffer, data, crc]);
}

export function encodeDmgBackgroundPng(frame = renderDmgBackgroundRgb()) {
  const { width, height, pixels } = frame;
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const phys = Buffer.alloc(9);
  phys.writeUInt32BE(DMG_BACKGROUND_PIXELS_PER_METER, 0);
  phys.writeUInt32BE(DMG_BACKGROUND_PIXELS_PER_METER, 4);
  phys[8] = 1;
  const scanline = width * 3;
  const raw = Buffer.alloc(height * (1 + scanline));
  for (let y = 0; y < height; y += 1) {
    const rowStart = y * (1 + scanline);
    raw[rowStart] = 0;
    pixels.copy(raw, rowStart + 1, y * scanline, (y + 1) * scanline);
  }
  const signature = Buffer.from([
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
  ]);
  return Buffer.concat([
    signature,
    pngChunk("IHDR", ihdr),
    pngChunk("pHYs", phys),
    pngChunk("IDAT", deflateSync(raw, { level: 9 })),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

export function dmgBackgroundDigest(png = encodeDmgBackgroundPng()) {
  return createHash("sha256").update(png).digest("hex");
}

function canonicalPath() {
  return join(ROOT, ...DMG_BACKGROUND_PATH.split("/"));
}

function parseArgs(args) {
  if (args.length === 0) return "preview";
  if (args.length === 1 && args[0] === "--apply") return "apply";
  if (args.length === 1 && args[0] === "--check") return "check";
  throw new Error(
    "Usage: node scripts/release/render-dmg-background.mjs [--apply|--check]",
  );
}

export function runDmgBackgroundCli(args, io = { writeFileSync, readFileSync }) {
  const mode = parseArgs(args);
  const png = encodeDmgBackgroundPng();
  const digest = dmgBackgroundDigest(png);
  if (mode === "preview") {
    return { mode, digest, path: DMG_BACKGROUND_PATH };
  }
  const path = canonicalPath();
  if (mode === "apply") {
    mkdirSync(dirname(path), { recursive: true });
    io.writeFileSync(path, png);
    return { mode, digest, path: DMG_BACKGROUND_PATH };
  }
  const existing = io.readFileSync(path);
  if (!Buffer.isBuffer(existing) || Buffer.compare(existing, png) !== 0) {
    throw new Error(
      `${DMG_BACKGROUND_PATH} does not match the deterministic V2 DMG renderer`,
    );
  }
  return { mode, digest, path: DMG_BACKGROUND_PATH };
}

if (isMain(import.meta.url)) {
  const result = runDmgBackgroundCli(process.argv.slice(2));
  process.stdout.write(`${result.digest} ${result.path}\n`);
}
