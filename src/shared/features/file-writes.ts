import * as z from "zod/mini";

// Paths are native-resolved display data, never accepted as mutation inputs.
const displayPath = z.string().check(
  z.minLength(1),
  z.maxLength(4096),
  z.refine((value) =>
    Array.from(value).every((character) => {
      const code = character.charCodeAt(0);
      return code > 31 && code !== 127;
    }),
  ),
);
export const fileWriteTargetSchema = z.strictObject({
  path: displayPath,
  backupPath: displayPath,
  exists: z.boolean(),
});

export type FileWriteTarget = z.infer<typeof fileWriteTargetSchema>;

export function parseFileWriteTarget(value: unknown): FileWriteTarget {
  const result = fileWriteTargetSchema.safeParse(value);
  if (!result.success) throw new Error("无法确认配置文件位置");
  return result.data;
}

export function parseFileDisplayPath(value: unknown): string {
  const result = displayPath.safeParse(value);
  if (!result.success) throw new Error("无法确认配置文件位置");
  return result.data;
}
