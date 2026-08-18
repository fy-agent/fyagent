# 前端设计：FyAgent 安装决策面板（四层不可静默变化）

## 1. 页面与布局
- 目标页面：安装详情页侧边栏块（与当前 InstallPanel 同级）。
- 结构：
  - 顶部：任务标题 + 当前状态总览 chip（`可安装 / 不可安装 / 需重确认`）
  - 四列/四行卡片：
    1. 来源与许可
    2. 包完整性
    3. 环境预检
    4. 安装计划
  - 底部：主行动区（`重新检查 / 确认重试 / 继续安装`）

## 2. 卡片视觉规范（高保真）
- 颜色：
  - ok: #2F6B46（绿）
  - warn: #9C6A0B（黄）
  - fail: #C62828（红）
  - unknown: #546E7A（灰）
- 组件：
  - 状态徽章（大字 + 小字）
  - 字段列表（label/value + metadata）
  - 展开区（默认收起）：原始证据 JSON（只读）
  - 错误码行（按优先级排序）

## 3. 交互行为

### 3.1 加载态
- 首屏显示骨架条 + “已读出厂参数”.
- 接口异常时显示“快照加载失败（离线可见缓存）”，并禁用安装。

### 3.2 状态行为
- `ok`：显示“可继续”，按钮允许点击。
- `warn`：显示“可继续，建议处理后重试”。
- `fail`：显示红色阻断；安装按钮禁用。
- `unknown`：显示“尚未确认”，安装禁用，暴露“重新检查”入口。

### 3.3 重确认交互（#28）
- 当 `snapshot_stale=true`：
  - 主按钮从 `继续安装` → `请先重生成安装计划`
  - 显示 Diff 清单（新增/移除/修改项）
  - 点击 `重生成安装计划`，完成后刷新状态并重新评估总状态。

### 3.4 可追溯性
- 每层显示：
  - `checked_at`
  - `evidence_from`（本机模块名）
  - `trace_id`（如有）
- 详情弹窗中支持“复制字段摘要（json）”供支持工单。

## 4. 文案规范
- 来源卡片文案：
  - “官方来源已核验 / 官方来源待确认 / 非官方来源”
- 完整性文案：
  - “签名验证：通过 / 未发现 / 未知”
  - “撤回状态：未撤回 / 已撤回 / 未确认”
- 环境预检文案：
  - “权限不足（需管理员）”“磁盘空间不足”“网络不可达”
- 计划文案：
  - “计划有变更，需重确认”（`snapshot_id` 已更新）

## 5. 组件拆分建议
- `InstallContractPanel`
  - `ContractSummaryHeader`
  - `SourceContractCard`
  - `IntegrityContractCard`
  - `EnvironmentContractCard`
  - `PlanContractCard`
  - `InstallActionBar`

## 6. 状态图（UI 侧）
```mermaid
flowchart TD
  A["打开安装详情"] --> B["加载 contract 快照"]
  B --> C{"有缺失字段?"}
  C -->|有| D["显示 unknown + 重试"]
  C -->|无| E{"source/fail/unknown?"}
  E -->|yes| F["按钮禁用 + 阻断说明"]
  E -->|no| G["检查 integrity"}
  G --> H{"integrity fail/unknown?"}
  H -->|yes| F
  H -->|no| I{"preflight fail/unknown?"}
  I -->|yes| F
  I -->|no| J{"plan_snapshot_stale?"}
  J -->|yes| K["重确认入口"]
  K --> L["重建快照后继续评估"]
  J -->|no| M["按钮可点，安装"]
  L --> M
```

## 7. 非功能需求
- 切换层级时不丢失 context（保留 session_id）。
- 所有卡片字段均支持 128% 字体可达，不出现水平滚动。
- 动画：仅折叠/展开与状态更新，不做过渡噪音。

## 8. 验收点（前端）
- 四层卡片全量可见且状态色一致。
- unknown 状态默认不显示可安装。
- plan stale 强制阻断并给出重确认按钮。
- 每层可展开查看证据与错误码来源。
