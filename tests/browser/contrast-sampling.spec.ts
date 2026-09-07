import { expect, test } from "@playwright/test";
import {
  sampleFilledControlContrast,
  sampleTextContrast,
} from "./support/visual";

test("contrast samples painted clipped glyph areas, not hidden ellipsis tails or unrelated fills", async ({
  page,
}) => {
  await page.setContent(`<main style="width:600px;background:white;padding:20px">
    <div style="position:relative;width:200px;background:#173047;color:white">
      <strong style="display:block;width:70px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis">long-clipped-address@example.test</strong>
      <span style="position:absolute;left:85px;top:0;width:300px;height:50px;background:white"></span>
    </div>
    <button style="display:block;margin-top:60px;background:#173047;color:white;border:1px solid #173047;padding:10px">Opaque action</button>
  </main>`);
  const samples = await sampleTextContrast(page, "main");
  expect(samples.some((sample) => sample.text.startsWith("long-clipped"))).toBe(
    true,
  );
  expect(samples.every((sample) => sample.ratio >= 4.5)).toBe(true);
  const fills = await sampleFilledControlContrast(page, "button");
  expect(fills).toHaveLength(1);
  expect(fills[0].ratio).toBeGreaterThan(3);
});
