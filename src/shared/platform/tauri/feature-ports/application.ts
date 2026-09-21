import { getVersion } from "@tauri-apps/api/app";

/** Read the running native package, which inherits the Cargo workspace version. */
export async function readAppVersion(): Promise<string> {
  const value: unknown = await getVersion();
  if (
    typeof value !== "string" ||
    value.length > 100 ||
    value.trim() !== value ||
    !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(
      value,
    )
  ) {
    throw new Error("Application version is unavailable");
  }
  return value;
}
