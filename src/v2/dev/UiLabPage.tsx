import { useState } from "react";

import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";
import { SparkleIcon } from "@phosphor-icons/react/dist/csr/Sparkle";

import { LiquidGlassLens } from "../shared/ui/LiquidGlassLens";
import { GlassButton, IconButton, Tooltip } from "../shared/ui/primitives";
import { SelectionLens, SelectionLensGroup } from "../shared/ui/SelectionLens";
import { PopoverPrimitive, TabsPrimitive } from "../shared/ui/vendor";

export function UiLabPage() {
  const [tab, setTab] = useState("one");

  return (
    <section className="fy-ui-lab" aria-labelledby="fy-ui-lab-title">
      <header className="fy-ui-lab-heading">
        <p className="fy-ui-lab-eyebrow">功能预览</p>
        <h1 id="fy-ui-lab-title">FyAgent 功能概览</h1>
      </header>

      <div className="fy-ui-lab-grid">
        <article className="fy-ui-lab-card">
          <h2>快速操作</h2>
          <div className="fy-ui-lab-row">
            <GlassButton data-testid="ui-lab-glass-button">
              <SparkleIcon size={18} weight="regular" aria-hidden />
              开始管理
            </GlassButton>

            <Tooltip label="查看操作说明" testId="ui-lab-tooltip-content">
              <IconButton
                aria-label="查看操作说明"
                data-testid="ui-lab-tooltip-trigger"
              >
                <CheckIcon size={18} weight="regular" aria-hidden />
              </IconButton>
            </Tooltip>

            <PopoverPrimitive.Root>
              <PopoverPrimitive.Trigger asChild>
                <GlassButton
                  aria-label="查看功能说明"
                  data-testid="ui-lab-popover-trigger"
                >
                  查看说明
                </GlassButton>
              </PopoverPrimitive.Trigger>
              <PopoverPrimitive.Portal>
                <PopoverPrimitive.Content
                  className="fy-popover"
                  sideOffset={10}
                  data-testid="ui-lab-popover-content"
                >
                  在此查看当前功能的使用说明。
                  <PopoverPrimitive.Arrow className="fy-popover-arrow" />
                </PopoverPrimitive.Content>
              </PopoverPrimitive.Portal>
            </PopoverPrimitive.Root>

            <GlassButton data-testid="ui-lab-focus-target">
              浏览功能
            </GlassButton>

            <LiquidGlassLens className="fy-ui-lab-lens-specimen">
              <span>推荐功能</span>
            </LiquidGlassLens>
          </div>
        </article>

        <article className="fy-ui-lab-card">
          <h2>功能信息</h2>
          <TabsPrimitive.Root
            className="fy-lab-tabs"
            value={tab}
            onValueChange={setTab}
          >
            <SelectionLensGroup id="ui-lab-tabs">
              <TabsPrimitive.List aria-label="功能状态">
                <TabsPrimitive.Trigger value="one">
                  <SelectionLens active={tab === "one"} />
                  <span>已启用</span>
                </TabsPrimitive.Trigger>
                <TabsPrimitive.Trigger value="two">
                  <SelectionLens active={tab === "two"} />
                  <span>待设置</span>
                </TabsPrimitive.Trigger>
              </TabsPrimitive.List>
            </SelectionLensGroup>
            <TabsPrimitive.Content value="one">
              已启用的功能会显示在这里。
            </TabsPrimitive.Content>
            <TabsPrimitive.Content value="two">
              完成设置后即可开始使用。
            </TabsPrimitive.Content>
          </TabsPrimitive.Root>
        </article>

        <article
          className="fy-ui-lab-card fy-ui-lab-token-surface"
          data-testid="ui-lab-token-surface"
        >
          <h2>账户信息</h2>
          <div className="fy-ui-lab-swatches" aria-label="账户状态">
            <span className="fy-swatch fy-swatch-accent" />
            <span className="fy-swatch fy-swatch-surface" />
            <span className="fy-swatch fy-swatch-border" />
          </div>
          <div className="fy-ui-lab-avatar" aria-label="账户">
            FY
          </div>
        </article>

        <article className="fy-ui-lab-card fy-ui-lab-copy">
          <h2>使用提示</h2>
          <p data-testid="ui-lab-long-labels">
            在 FyAgent 中集中管理应用、模型、Skills、MCP 服务、提示词和记忆。
          </p>
        </article>
      </div>
    </section>
  );
}
