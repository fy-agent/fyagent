import fs from "node:fs";
import path from "node:path";
import postcss from "postcss";
import { expect, it } from "vitest";

function cssFiles(directory: string): string[] {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const file = path.join(directory, entry.name);
    return entry.isDirectory()
      ? cssFiles(file)
      : file.endsWith(".css")
        ? [file]
        : [];
  });
}

it("keeps border-radius roles in tokens, including icon and separator exceptions", () => {
  const violations: string[] = [];
  for (const file of cssFiles("src/v2")) {
    const root = postcss.parse(fs.readFileSync(file, "utf8"), { from: file });
    root.walkDecls("border-radius", (declaration) => {
      if (declaration.value === "inherit" || declaration.value === "0") return;
      if (!declaration.value.includes("var(--fy-"))
        violations.push(
          `${file}:${declaration.source?.start?.line}: ${declaration.value}`,
        );
    });
  }
  expect(violations).toEqual([]);
});
