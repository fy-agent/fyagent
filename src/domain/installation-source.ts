/** Display-only source metadata; installer admission never accepts this URL. */
export function isDownloadSourceUrl(value: unknown): value is string {
  if (
    typeof value !== "string" ||
    value.length > 4096 ||
    Array.from(value).some((character) => {
      const code = character.charCodeAt(0);
      return code <= 32 || code === 127;
    })
  )
    return false;
  try {
    const url = new URL(value);
    return (
      url.protocol === "https:" &&
      url.hostname.length > 0 &&
      !url.username &&
      !url.password &&
      !url.hash
    );
  } catch {
    return false;
  }
}
