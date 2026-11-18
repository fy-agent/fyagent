import fs from "node:fs";
import { describe, expect, it } from "vitest";

describe("native security source-order regressions", () => {
  it("keeps the reviewed account and session identifiers out of diagnostic messages", () => {
    const production = (file: string) =>
      fs.readFileSync(`src-tauri/src/${file}`, "utf8").split("#[cfg(test)]")[0];
    for (const file of [
      "proxy/providers/copilot_auth.rs",
      "proxy/providers/codex_oauth_auth.rs",
    ]) {
      const source = production(file);
      expect(source).not.toContain("账号 {account_id}");
      expect(source).not.toContain("移除账号: {account_id}");
    }
    expect(production("proxy/handler_context.rs")).not.toContain(
      "Session ID: {}",
    );
    for (const [file, prefix] of [
      ["session_usage_opencode.rs", "OPENCODE"],
      ["session_usage_gemini.rs", "GEMINI"],
    ]) {
      const source = production(`services/${file}`);
      expect(source).not.toContain(`log::warn!("[${prefix}-SYNC] {msg}")`);
      expect(source).toContain("详细信息仅返回当前同步结果");
    }
    const grok = production("services/session_usage_grokbuild.rs");
    expect(grok).not.toContain("request_id={request_id}");
    expect(grok).not.toContain("插入失败 ({request_id})");
  });

  it("keeps the Windows package bridge handle-pinned and non-reparsing after ACL check removal", () => {
    const source = fs.readFileSync(
      "src-tauri/src/codex_desktop/platform/windows/package_bridge.rs",
      "utf8",
    );
    const production = source.split("#[cfg(test)]")[0];
    expect(production).toContain("OBJ_DONT_REPARSE");
    expect(production).toContain("FILE_OPEN_REPARSE_POINT");
    expect(production).not.toContain("fn verify_exact_descriptor(");
  });
});
