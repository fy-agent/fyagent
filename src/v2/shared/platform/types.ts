export type WindowPlatform =
  | "browser"
  | "windows"
  | "macos"
  | "linux"
  | "unknown";

export interface WindowFramePort {
  isNative: boolean;
  platform: WindowPlatform;
  prepareFrame(): Promise<void>;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
}

export type RuntimeEnvironment =
  | {
      isNative: false;
      platform: "browser";
    }
  | {
      isNative: true;
      platform: Exclude<WindowPlatform, "browser">;
    };
