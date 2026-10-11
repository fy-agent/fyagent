export function deepClone<T>(value: T): T {
  if (typeof globalThis.structuredClone === "function") {
    return globalThis.structuredClone(value);
  }

  return deepCloneFallback(value);
}

function deepCloneFallback<T>(value: T): T {
  if (value === null || typeof value !== "object") return value;
  if (value instanceof Date) return new Date(value.getTime()) as T;
  if (value instanceof Map) {
    return new Map(
      Array.from(value, ([key, entry]) => [
        deepCloneFallback(key),
        deepCloneFallback(entry),
      ]),
    ) as T;
  }
  if (Array.isArray(value)) {
    return value.map((item) => deepCloneFallback(item)) as T;
  }

  const cloned = {} as T;
  Object.keys(value).forEach((key) => {
    // 定义自有数据属性，保留合法键且不触发继承的 __proto__ setter。
    if (key === "__proto__") {
      Object.defineProperty(cloned, key, {
        value: deepCloneFallback(value[key as keyof T]),
        enumerable: true,
        configurable: true,
        writable: true,
      });
      return;
    }
    cloned[key as keyof T] = deepCloneFallback(value[key as keyof T]);
  });
  return cloned;
}
