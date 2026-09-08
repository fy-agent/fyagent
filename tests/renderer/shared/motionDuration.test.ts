import { afterEach, expect, it } from "vitest";
import {
  motionDuration,
  parseMotionDuration,
  fySpatialEase,
  fySpatialEasing,
  fyThemeRevealEase,
  fyThemeRevealEasing,
} from "@/shared/ui/motion";
import fs from "node:fs";

afterEach(() =>
  document.documentElement.style.removeProperty("--fy-motion-dialog-enter"),
);

it.each([
  ["420ms", 0.42],
  [".42s", 0.42],
  ["360ms", 0.36],
  ["0.36s", 0.36],
  ["80ms", 0.08],
  ["1e2ms", 0.1],
  [" 0s ", 0],
])(
  "parses CSS time %s without confusing seconds and milliseconds",
  (value, expected) => {
    expect(parseMotionDuration(value)).toBe(expected);
  },
);
it.each([
  "",
  "420",
  "-2s",
  "NaNms",
  "Infinitys",
  "1e999s",
  "calc(1s + 2ms)",
  "20ms, 1s",
  "20second",
])("fails closed for %s", (value) => {
  expect(parseMotionDuration(value)).toBe(0);
});
it("reads the same duration from source and optimized token syntax", () => {
  document.documentElement.style.setProperty(
    "--fy-motion-dialog-enter",
    "420ms",
  );
  expect(motionDuration("dialog-enter")).toBe(0.42);
  document.documentElement.style.setProperty(
    "--fy-motion-dialog-enter",
    ".42s",
  );
  expect(motionDuration("dialog-enter")).toBe(0.42);
});
it("keeps CSS and native/Motion spatial curves aligned", () => {
  const css = fs.readFileSync("src/app/styles/tokens.css", "utf8");
  const curve = css
    .match(/--fy-motion-ease:\s*cubic-bezier\(([^)]+)\)/)?.[1]
    .split(",")
    .map(Number);
  expect(curve).toEqual(fySpatialEase);
  expect(fySpatialEasing).toBe("cubic-bezier(0.32,0.72,0,1)");
});
it("keeps the shared theme-reveal curve aligned with its CSS token", () => {
  const css = fs.readFileSync("src/app/styles/tokens.css", "utf8");
  const curve = css
    .match(/--fy-motion-theme-ease:\s*cubic-bezier\(([^)]+)\)/)?.[1]
    .split(",")
    .map(Number);
  expect(curve).toEqual(fyThemeRevealEase);
  expect(fyThemeRevealEasing).toBe("cubic-bezier(0.25,0.08,0.25,1)");
});
