import { afterEach, describe, expect, it, vi } from "vitest";
import { deepClone } from "@/domain/configuration/serialization/deepClone";

describe("deepClone", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("preserves own __proto__ data consistently without changing prototypes", () => {
    const nativeClone = globalThis.structuredClone;
    expect(nativeClone).toBeTypeOf("function");
    const data = JSON.parse(
      '{"__proto__":{"polluted":true,"nested":{"value":"original"}},"nested":{"__proto__":{"polluted":true}},"list":[{"__proto__":{"polluted":true}}]}',
    ) as {
      __proto__: { polluted: boolean; nested: { value: string } };
      nested: { __proto__: { polluted: boolean } };
      list: { __proto__: { polluted: boolean } }[];
    };
    const source = {
      ...data,
      createdAt: new Date("2026-01-30T00:00:00.000Z"),
      entries: new Map([[data.nested, data.list[0]]]),
      optional: undefined,
    };
    const prototypeBefore = Object.getOwnPropertyDescriptors(Object.prototype);
    vi.stubGlobal("structuredClone", nativeClone);
    const native = deepClone(source);
    vi.stubGlobal("structuredClone", undefined);
    const fallback = deepClone(source);

    expect(fallback).toStrictEqual(native);
    for (const result of [native, fallback]) {
      expect(Reflect.ownKeys(result)).toEqual(Reflect.ownKeys(source));
      expect(result.createdAt).not.toBe(source.createdAt);
      expect(result.entries).not.toBe(source.entries);
      const [mapKey, mapValue] = [...result.entries][0];
      expect(mapKey).not.toBe(data.nested);
      expect(mapValue).not.toBe(data.list[0]);
      for (const object of [
        result,
        result.nested,
        result.list[0],
        mapKey,
        mapValue,
      ]) {
        expect(Object.prototype.hasOwnProperty.call(object, "__proto__")).toBe(
          true,
        );
        expect(Object.getPrototypeOf(object)).toBe(Object.prototype);
        expect(Object.prototype.hasOwnProperty.call(object, "polluted")).toBe(
          false,
        );
        expect("polluted" in object).toBe(false);
        expect(
          Object.getOwnPropertyDescriptor(object, "__proto__"),
        ).toMatchObject({
          enumerable: true,
          configurable: true,
          writable: true,
        });
      }
      expect(result.__proto__).not.toBe(source.__proto__);
      expect(result.__proto__.nested).not.toBe(source.__proto__.nested);
      expect(result.nested.__proto__).not.toBe(source.nested.__proto__);
      expect(result.list[0].__proto__).not.toBe(source.list[0].__proto__);
      result.__proto__.nested.value = "changed";
      expect(source.__proto__.nested.value).toBe("original");
    }
    expect(Object.getOwnPropertyDescriptors(Object.prototype)).toEqual(
      prototypeBefore,
    );
  });

  it("falls back when structuredClone is unavailable", () => {
    vi.stubGlobal("structuredClone", undefined);

    const source = {
      nested: { value: "original" },
      list: [{ enabled: true }],
      createdAt: new Date("2026-01-30T00:00:00.000Z"),
    };

    const cloned = deepClone(source);
    cloned.nested.value = "changed";
    cloned.list[0].enabled = false;

    expect(cloned).not.toBe(source);
    expect(cloned.nested).not.toBe(source.nested);
    expect(cloned.list).not.toBe(source.list);
    expect(cloned.createdAt).not.toBe(source.createdAt);
    expect(cloned.createdAt.getTime()).toBe(source.createdAt.getTime());
    expect(source.nested.value).toBe("original");
    expect(source.list[0].enabled).toBe(true);
  });
});
