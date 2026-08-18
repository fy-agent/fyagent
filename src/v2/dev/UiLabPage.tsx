import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";
import { SparkleIcon } from "@phosphor-icons/react/dist/csr/Sparkle";

import { GlassButton, IconButton, Tooltip } from "../shared/ui/primitives";
import { PopoverPrimitive, TabsPrimitive } from "../shared/ui/vendor";

export function UiLabPage() {
  return (
    <section className="fy-ui-lab" aria-labelledby="fy-ui-lab-title">
      <header className="fy-ui-lab-heading">
        <p className="fy-ui-lab-eyebrow">Development surface</p>
        <h1 id="fy-ui-lab-title">FyAgent UI Lab</h1>
      </header>

      <div className="fy-ui-lab-grid">
        <article className="fy-ui-lab-card">
          <h2>Controls</h2>
          <div className="fy-ui-lab-row">
            <GlassButton data-testid="ui-lab-glass-button">
              <SparkleIcon size={18} weight="regular" aria-hidden />
              玻璃按钮
            </GlassButton>

            <Tooltip label="Tooltip 已打开" testId="ui-lab-tooltip-content">
              <IconButton
                aria-label="Tooltip 示例"
                data-testid="ui-lab-tooltip-trigger"
              >
                <CheckIcon size={18} weight="regular" aria-hidden />
              </IconButton>
            </Tooltip>

            <PopoverPrimitive.Root>
              <PopoverPrimitive.Trigger asChild>
                <GlassButton
                  aria-label="打开 Popover"
                  data-testid="ui-lab-popover-trigger"
                >
                  打开 Popover
                </GlassButton>
              </PopoverPrimitive.Trigger>
              <PopoverPrimitive.Portal>
                <PopoverPrimitive.Content
                  className="fy-popover"
                  sideOffset={10}
                  data-testid="ui-lab-popover-content"
                >
                  Portal 内容位于内容面板之上。
                  <PopoverPrimitive.Arrow className="fy-popover-arrow" />
                </PopoverPrimitive.Content>
              </PopoverPrimitive.Portal>
            </PopoverPrimitive.Root>

            <GlassButton data-testid="ui-lab-focus-target">
              键盘焦点
            </GlassButton>
          </div>
        </article>

        <article className="fy-ui-lab-card">
          <h2>Tabs candidate</h2>
          <TabsPrimitive.Root className="fy-lab-tabs" defaultValue="one">
            <TabsPrimitive.List aria-label="UI Lab Tabs">
              <TabsPrimitive.Trigger value="one">稳定面</TabsPrimitive.Trigger>
              <TabsPrimitive.Trigger value="two">透明面</TabsPrimitive.Trigger>
            </TabsPrimitive.List>
            <TabsPrimitive.Content value="one">
              Radix keyboard behavior with FyAgent-owned tokens.
            </TabsPrimitive.Content>
            <TabsPrimitive.Content value="two">
              Glass remains restrained and readable.
            </TabsPrimitive.Content>
          </TabsPrimitive.Root>
        </article>

        <article
          className="fy-ui-lab-card fy-ui-lab-token-surface"
          data-testid="ui-lab-token-surface"
        >
          <h2>Token surface</h2>
          <div className="fy-ui-lab-swatches" aria-label="语义化颜色">
            <span className="fy-swatch fy-swatch-accent" />
            <span className="fy-swatch fy-swatch-surface" />
            <span className="fy-swatch fy-swatch-border" />
          </div>
          <div className="fy-ui-lab-avatar" aria-label="Avatar 占位">
            FY
          </div>
        </article>

        <article className="fy-ui-lab-card fy-ui-lab-copy">
          <h2>Long-label pressure</h2>
          <p data-testid="ui-lab-long-labels">
            Agent catalogue and workspace orchestration controls ·
            エージェントカタログとワークスペースオーケストレーション ·
            智能体目录与工作区编排控制
          </p>
        </article>
      </div>
    </section>
  );
}
