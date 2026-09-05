import { describe, expect, it } from "vitest";
import {
  deepMerge,
  deepRemove,
  sanitizeSnippet,
} from "@/utils/providerConfigStructural";

describe("provider structural configuration ownership", () => {
  it("does not merge into inherited objects or execute inherited setters", () => {
    const inherited = { nested: { original: true } };
    let setterCalled = false;
    Object.defineProperty(inherited, "setting", {
      set() {
        setterCalled = true;
      },
    });
    const target = Object.create(inherited);
    deepMerge(target, { nested: { added: true }, setting: "owned" });
    expect(inherited.nested).toEqual({ original: true });
    expect(target.nested).toEqual({ added: true });
    expect(Object.prototype.hasOwnProperty.call(target, "nested")).toBe(true);
    expect(Object.prototype.hasOwnProperty.call(target, "setting")).toBe(true);
    expect(setterCalled).toBe(false);
  });

  it("does not remove inherited values and rejects nested pollution keys", () => {
    const inherited = { nested: { original: true } };
    deepRemove(Object.create(inherited), { nested: { original: true } });
    expect(inherited.nested).toEqual({ original: true });
    const input = JSON.parse(
      '{"nested":{"__proto__":{"polluted":true},"constructor":{"prototype":{"polluted":true}},"safe":1}}',
    );
    expect(sanitizeSnippet(input)).toEqual({ nested: { safe: 1 } });
    expect(deepMerge({}, input)).toEqual({ nested: { safe: 1 } });
    expect(
      Object.getOwnPropertyDescriptor(Object.prototype, "polluted"),
    ).toBeUndefined();
  });

  it("preserves recursive merge and replacement-array semantics", () => {
    const target = { nested: { keep: 1, change: 2 }, items: [1, 2] };
    expect(deepMerge(target, { nested: { change: 3 }, items: [4] })).toBe(
      target,
    );
    expect(target).toEqual({ nested: { keep: 1, change: 3 }, items: [4] });
    deepRemove(target, { nested: { change: 3 }, items: [4] });
    expect(target).toEqual({ nested: { keep: 1 } });
  });
});
