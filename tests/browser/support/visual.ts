import type { Page } from "@playwright/test";

/** Supplemental raster samples for gradients/backdrop layers that axe marks
 * incomplete. This is WCAG relative luminance arithmetic, not an a11y engine.
 * Samples are finite and never constitute whole-application certification. */
export function sampleTextContrast(page: Page, selector: string) {
  return sampleContrast(page, selector, "text");
}

export function sampleControlBoundaryContrast(page: Page, selector: string) {
  return sampleContrast(page, selector, "boundary");
}

/** Opaque filled buttons are identified by their whole silhouette, not by a
 * border contrasted against their identical fill. Inputs still use both sides. */
export function sampleFilledControlContrast(page: Page, selector: string) {
  return sampleContrast(page, selector, "fill");
}

async function sampleContrast(
  page: Page,
  selector: string,
  kind: "text" | "boundary" | "fill",
) {
  const records = await page.evaluate(
    ({ selector, kind }) => {
      const root = document.querySelector(selector);
      if (!root) throw new Error(`Missing contrast scope ${selector}`);
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      const records: {
        text: string;
        color: number[];
        opacity: number;
        points: number[][];
        adjacent?: number[][];
      }[] = [];
      if (kind !== "text") {
        for (const element of document.querySelectorAll<HTMLElement>(
          selector,
        )) {
          if (
            element.closest(
              '[hidden], [inert], :disabled, [aria-disabled="true"]',
            )
          )
            continue;
          const rect = element.getBoundingClientRect();
          if (
            !rect.width ||
            rect.top < 2 ||
            rect.bottom > innerHeight ||
            rect.right > innerWidth
          )
            continue;
          const style = getComputedStyle(element);
          const color = (
            kind === "fill" ? style.backgroundColor : style.borderTopColor
          )
            .match(/[\d.]+/g)
            ?.map(Number);
          if (!color || parseFloat(style.borderTopWidth) < 1) continue;
          if (kind === "fill" && (color[3] ?? 1) !== 1)
            throw new Error("Filled contrast requires an opaque control");
          const x = rect.x + rect.width / 2;
          records.push({
            text: `${element.tagName} boundary`,
            color,
            opacity: color[3] ?? 1,
            points: [[x, rect.top + 0.5]],
            adjacent:
              kind === "fill"
                ? [[x, rect.top - 2]]
                : [
                    [x, rect.top - 2],
                    [x, rect.top + 2],
                  ],
          });
        }
      }
      while (kind === "text" && walker.nextNode()) {
        const text = walker.currentNode;
        const parent = text.parentElement;
        if (
          !text.textContent?.trim() ||
          !parent ||
          parent.closest(
            '[hidden], [inert], [aria-hidden="true"], :disabled, [aria-disabled="true"]',
          )
        )
          continue;
        const range = document.createRange();
        range.selectNodeContents(text);
        const rangeRect = Array.from(range.getClientRects()).find(
          (rect) => rect.width > 2 && rect.height > 2,
        );
        if (!rangeRect) continue;
        // Range geometry includes text hidden by ellipsis/overflow. Sampling
        // that unpainted tail measures a neighbouring badge or pane instead.
        let left = Math.max(0, rangeRect.left);
        let right = Math.min(innerWidth, rangeRect.right);
        let top = Math.max(0, rangeRect.top);
        let bottom = Math.min(innerHeight, rangeRect.bottom);
        for (
          let ancestor: HTMLElement | null = parent;
          ancestor;
          ancestor = ancestor.parentElement
        ) {
          const style = getComputedStyle(ancestor);
          const bounds = ancestor.getBoundingClientRect();
          if (style.overflowX !== "visible") {
            left = Math.max(left, bounds.left);
            right = Math.min(right, bounds.right);
          }
          if (style.overflowY !== "visible") {
            top = Math.max(top, bounds.top);
            bottom = Math.min(bottom, bounds.bottom);
          }
        }
        if (right - left <= 2 || bottom - top <= 2) continue;
        const rect = {
          x: left,
          y: top,
          width: right - left,
          height: bottom - top,
        };
        const painted = document.elementFromPoint(
          rect.x + rect.width / 2,
          rect.y + rect.height / 2,
        );
        if (
          !painted ||
          (!parent.contains(painted) && !painted.contains(parent))
        )
          continue;
        const color = getComputedStyle(parent)
          .color.match(/[\d.]+/g)
          ?.map(Number);
        if (!color || color.length < 3) continue;
        let opacity = color[3] ?? 1;
        for (
          let element: HTMLElement | null = parent;
          element;
          element = element.parentElement
        )
          opacity *= Number(getComputedStyle(element).opacity);
        records.push({
          text: text.textContent.trim().slice(0, 50),
          color,
          opacity,
          points: [0.15, 0.5, 0.85].map((fraction) => [
            rect.x + fraction * rect.width,
            rect.y + rect.height / 2,
          ]),
        });
      }
      return records;
    },
    { selector, kind },
  );
  const hiding = await page.addStyleTag({
    content:
      kind === "text"
        ? `${selector}, ${selector} * { -webkit-text-fill-color: transparent !important; text-shadow: none !important; }`
        : `${selector} { border-color: transparent !important; outline-color: transparent !important; box-shadow: none !important; }`,
  });
  let image: string;
  try {
    image = (
      await page.screenshot({ scale: "css", animations: "disabled" })
    ).toString("base64");
  } finally {
    await hiding.evaluate((element) =>
      element.parentNode?.removeChild(element),
    );
  }
  return page.evaluate(
    async ({ image, records }) => {
      const bitmap = new Image();
      bitmap.src = `data:image/png;base64,${image}`;
      await bitmap.decode();
      const canvas = document.createElement("canvas");
      canvas.width = bitmap.width;
      canvas.height = bitmap.height;
      const context = canvas.getContext("2d", { willReadFrequently: true });
      if (!context) throw new Error("Raster contrast sampling requires canvas");
      context.drawImage(bitmap, 0, 0);
      const luminance = (rgb: number[]) =>
        rgb
          .map((byte) => {
            const value = byte / 255;
            return value <= 0.04045
              ? value / 12.92
              : ((value + 0.055) / 1.055) ** 2.4;
          })
          .reduce(
            (sum, value, index) =>
              sum + value * [0.2126, 0.7152, 0.0722][index],
            0,
          );
      return records.map((record) => ({
        text: record.text,
        ratio: Math.min(
          ...record.points.map(([x, y]) => {
            const background = Array.from(
              context.getImageData(Math.floor(x), Math.floor(y), 1, 1).data,
            ).slice(0, 3);
            const foreground = record.color
              .slice(0, 3)
              .map(
                (channel, index) =>
                  channel * record.opacity +
                  background[index] * (1 - record.opacity),
              );
            const backgrounds = record.adjacent?.map(([sx, sy]) =>
              Array.from(
                context.getImageData(Math.floor(sx), Math.floor(sy), 1, 1).data,
              ).slice(0, 3),
            ) ?? [background];
            return Math.min(
              ...backgrounds.map((adjacent) => {
                const values = [
                  luminance(adjacent),
                  luminance(foreground),
                ].sort((a, b) => a - b);
                return (values[1] + 0.05) / (values[0] + 0.05);
              }),
            );
          }),
        ),
      }));
    },
    { image, records },
  );
}
