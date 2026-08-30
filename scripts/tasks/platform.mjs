export function isPosixTaskHost(platform) {
  return platform === "darwin" || platform === "linux";
}
