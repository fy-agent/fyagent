import { describe, expect, it } from "vitest";
import {
  assertChangelogMatchesVersion,
  readCargoWorkspaceVersion,
} from "../scripts/release/release-contract.mjs";

describe("changelog release contract", () => {
  it("reads the unique workspace.package version", () => {
    expect(
      readCargoWorkspaceVersion(`
[workspace]
members = [".", "user-helper"]

[workspace.package]
version = "0.4.2"
license = "MIT"

[package]
name = "fyagent"
version.workspace = true
`),
    ).toBe("0.4.2");
  });

  it("accepts a Keep a Changelog file whose first version heading matches", () => {
    expect(() =>
      assertChangelogMatchesVersion(
        `# Changelog

## [0.4.2] - 2026-08-21

Styled the macOS DMG window.

## [0.4.1] - 2026-08-20

Previous notes.
`,
        "0.4.2",
      ),
    ).not.toThrow();
  });

  it("rejects a missing current-version heading", () => {
    expect(() =>
      assertChangelogMatchesVersion(
        `# Changelog

## [0.4.1] - 2026-08-20

Notes.
`,
        "0.4.2",
      ),
    ).toThrow(
      "CHANGELOG.md must start its version history with ## [0.4.2] - YYYY-MM-DD",
    );
  });

  it("rejects an empty current-version body", () => {
    expect(() =>
      assertChangelogMatchesVersion(
        `# Changelog

## [0.4.2] - 2026-08-21

<!-- leftover -->

## [0.4.1] - 2026-08-20

Notes.
`,
        "0.4.2",
      ),
    ).toThrow(
      "CHANGELOG.md heading for 0.4.2 must be followed by non-empty notes",
    );
  });

  it("rejects a version that does not match Cargo", () => {
    expect(() =>
      assertChangelogMatchesVersion(
        `## [0.4.3] - 2026-08-21

Notes.
`,
        "0.4.2",
      ),
    ).toThrow(
      "CHANGELOG.md must start its version history with ## [0.4.2] - YYYY-MM-DD",
    );
  });
});
