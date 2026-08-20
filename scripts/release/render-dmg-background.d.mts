export const DMG_BACKGROUND_PATH: "src-tauri/icons/dmg-background.png";
export const DMG_WINDOW_WIDTH_PT: 660;
export const DMG_WINDOW_HEIGHT_PT: 400;
export const DMG_BACKGROUND_SCALE: 2;
export const DMG_BACKGROUND_WIDTH: number;
export const DMG_BACKGROUND_HEIGHT: number;
export const DMG_BACKGROUND_PIXELS_PER_METER: 5669;
export const DMG_ICON_SIZE_PT: 128;
export const DMG_APP_XY: readonly [number, number];
export const DMG_APPLICATIONS_XY: readonly [number, number];

export function renderDmgBackgroundRgb(): {
  width: number;
  height: number;
  pixels: Buffer;
};
export function encodeDmgBackgroundPng(frame?: {
  width: number;
  height: number;
  pixels: Buffer;
}): Buffer;
export function dmgBackgroundDigest(png?: Buffer): string;
export function runDmgBackgroundCli(
  args: string[],
  io?: {
    writeFileSync: (path: string, data: Buffer) => void;
    readFileSync: (path: string) => Buffer | string;
  },
): { mode: "preview" | "apply" | "check"; digest: string; path: string };
