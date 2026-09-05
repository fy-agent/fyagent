import { parse } from "parse5";

/** Actual HTML elements with source ranges; inert template contents are excluded. */
export function htmlElements(source, names) {
  const pending = [parse(source, { sourceCodeLocationInfo: true })];
  const elements = [];
  while (pending.length) {
    const node = pending.pop();
    if (
      node.namespaceURI === "http://www.w3.org/1999/xhtml" &&
      names.includes(node.tagName) &&
      node.sourceCodeLocation
    ) {
      elements.push(node);
    }
    for (const child of node.childNodes ?? []) pending.push(child);
  }
  return elements.sort(
    (left, right) =>
      left.sourceCodeLocation.startOffset -
      right.sourceCodeLocation.startOffset,
  );
}

export function htmlAttribute(element, name) {
  return element.attrs.find((attribute) => attribute.name === name)?.value;
}

export function scriptContent(source, element) {
  const { startTag, endTag } = element.sourceCodeLocation;
  if (!startTag || !endTag) {
    throw new Error("Unclosed script element in dist/index.html.");
  }
  return source.slice(startTag.endOffset, endTag.startOffset);
}
