import type { DefaultTreeAdapterTypes } from "parse5";

export type LocatedHtmlElement = DefaultTreeAdapterTypes.Element & {
  sourceCodeLocation: NonNullable<
    DefaultTreeAdapterTypes.Element["sourceCodeLocation"]
  >;
};

export function htmlElements(
  source: string,
  names: readonly string[],
): LocatedHtmlElement[];
export function htmlAttribute(
  element: DefaultTreeAdapterTypes.Element,
  name: string,
): string | undefined;
export function scriptContent(
  source: string,
  element: LocatedHtmlElement,
): string;
