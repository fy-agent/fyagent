export type WindowPlatform = "browser" | "windows" | "macos" | "unknown";

export type RuntimeEnvironment =
  | {
      isNative: false;
      platform: "browser";
    }
  | {
      isNative: true;
      platform: Exclude<WindowPlatform, "browser">;
    };
