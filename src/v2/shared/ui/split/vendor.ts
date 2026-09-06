// Keep the resize library at its feature boundary; exporting it from the
// ubiquitous Radix facade would eagerly load it with window chrome.
export {
  Group as ResizableGroup,
  Panel as ResizablePanel,
  Separator as ResizeSeparator,
} from "react-resizable-panels";
export type { PanelImperativeHandle } from "react-resizable-panels";
