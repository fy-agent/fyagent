---
type: asset-audit
status: current
updated: 2026-08-10
review_on: 2026-09-10
authority: fyagent-docs
source: docs/user-manual/assets
---

# 用户手册截图审计

## 结论

当前目录有 40 张 PNG，三语正文一共引用 84 次：18 张按中、英、日分别制作的 Claude Desktop 截图各引用 1 次，另外 22 张无语言后缀的截图各被三种语言引用 1 次。

这些图都来自 CC Switch 时期的界面。抽查覆盖主界面、供应商、Skills、用量和 Claude Desktop 三语版本，能直接看到 `CC Switch`、旧导航、旧应用范围或中文界面复用于英文和日文手册。它们可以暂时帮助读者辨认大致位置，但不能作为当前 FyAgent 的运行时证据，也不应进入 README 或营销页面。

本次统一结论为 `replace_required`：文件和现有引用暂不删除，避免手册突然失去操作上下文；按[拍摄任务卡](../../user-manual/assets/shot-cards/README.md)获得真实 FyAgent 运行时截图后再替换。18 张本地化截图必须保持同语种替换；22 张共享截图含界面文字，新的英文和日文手册也应分别拍摄，不再复用中文图。

证据等级为 `code_audit + sampled_visual_review`，不是 `runtime_screenshot`。SHA-256 一栏为前 12 位，便于后续确认文件是否变化。

## 资产与引用总表

| 文件 | 当前语言 | 尺寸 | SHA-256 | 引用数 | 引用章节 | 结论 |
|---|---|---:|---|---:|---|---|
| `claude-desktop-add-provider-en.png` | en | 1280×720 | `6cbb0077e007` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-add-provider-ja.png` | ja | 1280×720 | `f7c307f1230b` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-add-provider.png` | zh | 1280×720 | `fc398e8b8bf6` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-import-from-claude-en.png` | en | 1280×720 | `1a3eacec1085` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-import-from-claude-ja.png` | ja | 1280×720 | `bf903e904f4e` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-import-from-claude.png` | zh | 1280×720 | `5c2c9bab3c00` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-model-mapping-rows-en.png` | en | 1280×720 | `ddacf8492fe8` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-model-mapping-rows-ja.png` | ja | 1280×720 | `d962e4130640` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-model-mapping-rows.png` | zh | 1280×720 | `30e66fc31fcf` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-panel-en.png` | en | 1280×720 | `08f81c24d7c7` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-panel-ja.png` | ja | 1280×720 | `0185bc068dd6` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-panel.png` | zh | 1280×720 | `828745cb9206` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `claude-desktop-route-toggle-context-en.png` | en | 140×58 | `fea9db8f21af` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍；扩大上下文 |
| `claude-desktop-route-toggle-context-ja.png` | ja | 140×58 | `5c9a9a5a88f1` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍；扩大上下文 |
| `claude-desktop-route-toggle-context.png` | zh | 220×62 | `a45f996b11be` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍；扩大上下文 |
| `image-20260108001629138.png` | 共享中文 | 2202×1502 | `146f3f6635d9` | 3 | `1-getting-started/1.3-interface.md` | 三语分别重拍；主界面可见旧品牌 |
| `image-20260108002153668.png` | 共享中文 | 532×616 | `7d694cd01a75` | 3 | `1-getting-started/1.3-interface.md` | 三语分别重拍 |
| `image-20260108002626389.png` | 共享中文 | 2202×1502 | `4af1cd8cec60` | 3 | `1-getting-started/1.4-quickstart.md` | 三语分别重拍 |
| `image-20260108002807657.png` | 共享中文 | 2202×1502 | `c272ccdd2b95` | 3 | `1-getting-started/1.4-quickstart.md` | 三语分别重拍 |
| `image-20260108004348993.png` | 共享中文 | 532×748 | `a69e9cc783e7` | 3 | `3-providers/3.2-switch.md` | 三语分别重拍 |
| `image-20260108004734882.png` | 共享中文 | 2202×1502 | `e241fe142db1` | 3 | `3-providers/3.3-edit.md` | 三语分别重拍 |
| `image-20260108004946288.png` | 共享中文 | 2202×1502 | `60fbc7b91be2` | 3 | `3-providers/3.4-sort-duplicate.md` | 三语分别重拍 |
| `image-20260108005327817.png` | 共享中文 | 2202×1502 | `17b1402f3891` | 3 | `3-providers/3.1-add.md` | 三语分别重拍；示例地址需换成中性数据 |
| `image-20260108005723522.png` | 共享中文 | 2202×1502 | `1f29b2da7e44` | 3 | `4-extensions/4.1-mcp.md` | 三语分别重拍 |
| `image-20260108005739731.png` | 共享中文 | 2202×1502 | `e5243c07b1a0` | 3 | `4-extensions/4.1-mcp.md` | 三语分别重拍 |
| `image-20260108010110382.png` | 共享中文 | 2202×1502 | `c47690279818` | 3 | `4-extensions/4.2-prompts.md` | 三语分别重拍 |
| `image-20260108010253926.png` | 共享中文 | 2202×1502 | `1903b56fc722` | 3 | `4-extensions/4.3-skills.md` | 三语分别重拍；应用范围已变化 |
| `image-20260108010308060.png` | 共享中文 | 2202×1502 | `382c48d10673` | 3 | `4-extensions/4.3-skills.md` | 三语分别重拍 |
| `image-20260108010324583.png` | 共享中文 | 2202×1502 | `69828dc8c269` | 3 | `4-extensions/4.3-skills.md` | 三语分别重拍 |
| `image-20260108011338922.png` | 共享中文 | 2202×1502 | `cdcb094262b7` | 3 | `5-proxy/5.1-service.md` | 三语分别重拍 |
| `image-20260108011353927.png` | 共享中文 | 2202×1502 | `f35bdec6eec7` | 3 | `5-proxy/5.1-service.md` | 三语分别重拍 |
| `image-20260108011730105.png` | 共享中文 | 2202×1502 | `0f29d7e6b7bc` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍；旧统计布局 |
| `image-20260108011742847.png` | 共享中文 | 2202×1502 | `95b15550d1fa` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍 |
| `image-20260108011859974.png` | 共享中文 | 2202×1502 | `0efdccdcc2b0` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍 |
| `image-20260108011907928.png` | 共享中文 | 2202×1502 | `79bc7e8e287e` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍 |
| `image-20260108011915381.png` | 共享中文 | 2202×1502 | `8e2ee4babe70` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍 |
| `image-20260108011933565.png` | 共享中文 | 2202×1502 | `dd83553683df` | 3 | `5-proxy/5.4-usage.md` | 三语分别重拍 |
| `local-routing-display-setting-en.png` | en | 1280×720 | `519b0d24bfe1` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `local-routing-display-setting-ja.png` | ja | 1280×720 | `301d945b8aee` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |
| `local-routing-display-setting.png` | zh | 1280×720 | `916ea3aa0f9d` | 1 | `3-providers/3.6-claude-desktop.md` | 同语种重拍 |

## 引用处置

表中每一行的结论同时适用于该文件的全部引用：`18 × 1 + 22 × 3 = 84`。重拍时先完成中文基准图，再用相同窗口尺寸、数据和交互状态拍摄英文、日文版本；任何含密钥、用户名、真实路径、账号、内网地址或业务数据的画面都不入库。

README 的 6 张旧截图已经停止引用。当前主机在 Visual Studio 2022 Developer PowerShell 中已经通过 `cl.exe` 与 WebView2 预检，但本轮仍没有把未经三语数据固定和脱敏复核的临时画面发布为 proof frame，也没有用网页预览或生成图冒充真实应用截图。
