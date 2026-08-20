export interface CanonicalPngResult {
  readonly width: number;
  readonly height: number;
  readonly transparent: number;
  readonly partial: number;
  readonly opaque: number;
  readonly signal: number;
  readonly digest: string;
}

export interface TrayPngResult {
  readonly width: number;
  readonly height: number;
  readonly bounds: readonly [number, number, number, number];
  readonly digest: string;
}

export interface IcoFrame {
  readonly width: number;
  readonly height: number;
  readonly colorCount: number;
  readonly bitCount: number;
  readonly digest: string;
}

export function decodeRgbaPng(
  buffer: Buffer,
  label?: string,
): { readonly width: number; readonly height: number; readonly pixels: Buffer };
export function validateCanonicalPng(
  buffer: Buffer,
  label?: string,
): CanonicalPngResult;
export function validateTrayPng(
  buffer: Buffer,
  expectedSize: number,
  label: string,
): TrayPngResult;
export function parseIcoFrames(buffer: Buffer, label?: string): IcoFrame[];
export function applyApplicationBrandAssets(): unknown;
export function checkApplicationBrandAssets(): unknown;
