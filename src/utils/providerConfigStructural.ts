/** Internal structural helpers shared by JSON and TOML provider config owners. */

export const isPlainObject = (value: unknown): value is Record<string, any> =>
  Object.prototype.toString.call(value) === "[object Object]";

const FORBIDDEN_MERGE_KEYS = new Set(["__proto__", "constructor", "prototype"]);

function setOwnValue(
  target: Record<string, any>,
  key: string,
  value: any,
): void {
  Object.defineProperty(target, key, {
    value,
    enumerable: true,
    configurable: true,
    writable: true,
  });
}

/**
 * Strip prototype-pollution keys before structural read-side comparison.
 * Write-side JSON operations keep their own defensive key checks too.
 */
export const sanitizeSnippet = (value: any): any => {
  if (Array.isArray(value)) return value.map(sanitizeSnippet);
  if (!isPlainObject(value)) return value;

  const cleaned: Record<string, any> = {};
  for (const [key, child] of Object.entries(value)) {
    if (FORBIDDEN_MERGE_KEYS.has(key)) continue;
    setOwnValue(cleaned, key, sanitizeSnippet(child));
  }
  return cleaned;
};

export const isSubset = (target: any, source: any): boolean => {
  if (isPlainObject(source)) {
    if (!isPlainObject(target)) return false;
    return Object.entries(source).every(([key, value]) => {
      if (FORBIDDEN_MERGE_KEYS.has(key)) return false;
      if (!Object.prototype.hasOwnProperty.call(target, key)) return false;
      return isSubset(target[key], value);
    });
  }

  if (Array.isArray(source)) {
    if (!Array.isArray(target) || target.length !== source.length) return false;
    return source.every((item, index) => isSubset(target[index], item));
  }

  return target === source;
};

export const deepMerge = (
  target: Record<string, any>,
  source: Record<string, any>,
): Record<string, any> => {
  Object.entries(source).forEach(([key, value]) => {
    if (FORBIDDEN_MERGE_KEYS.has(key)) return;

    if (isPlainObject(value)) {
      if (
        !Object.prototype.hasOwnProperty.call(target, key) ||
        !isPlainObject(target[key])
      )
        setOwnValue(target, key, {});
      deepMerge(target[key], value);
    } else {
      setOwnValue(target, key, value);
    }
  });
  return target;
};

export const deepRemove = (
  target: Record<string, any>,
  source: Record<string, any>,
): void => {
  Object.entries(source).forEach(([key, value]) => {
    if (FORBIDDEN_MERGE_KEYS.has(key)) return;
    if (!Object.prototype.hasOwnProperty.call(target, key)) return;

    if (isPlainObject(value) && isPlainObject(target[key])) {
      deepRemove(target[key], value);
      if (Object.keys(target[key]).length === 0) delete target[key];
    } else if (isSubset(target[key], value)) {
      delete target[key];
    }
  });
};
