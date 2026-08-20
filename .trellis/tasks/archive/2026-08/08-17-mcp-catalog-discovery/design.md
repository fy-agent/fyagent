# MCP 精选发现与安装 — 技术设计

## Architecture and boundaries

Catalog 是产品层安装配方，`McpServer` 是持久化运行配置。两者分离：

- Catalog 元数据（分类、厂商、来源、风险、表单 schema）只存在前端静态模块。
- 安装结果只调用现有 `McpPort.upsert`，写入当前统一 MCP 结构。
- 不新增 Tauri 命令、数据库字段或远程市场 port。

层边界：

```text
pages/mcp (Page, Discovery, InstallDialog, catalog)
  -> shared/features (helpers, presets, types, mcpSecurity, mcpLaunch)
  -> shared/platform feature ports
```

V2 不得从 `src/components`、`src/hooks`、`src/lib`、`src/i18n` 导入。CSS 使用 `.fy-mcp-*` / `.fy-feature-*` 与 `--fy-*` token。

## Module split

| Module | Responsibility |
| --- | --- |
| `src/v2/shared/features/mcpSecurity.ts` | URL query 脱敏、敏感 args 遮罩、安全展示文本 |
| `src/v2/shared/features/helpers.ts` | `buildMcpSearchText` 改用安全字段；不索引 env/headers，不索引 URL query 值 |
| `src/v2/shared/features/mcpLaunch.ts` | `buildNpxCommand(packageName, platform)`：Windows `cmd /c npx -y`，其他直接 `npx -y` |
| `src/v2/shared/features/presets.ts` | Time 改为 uvx；npx 模板改走 `mcpLaunch`；unknown 不再丢弃 npx 模板 |
| `src/v2/pages/mcp/catalog.ts` | 静态精选、typed builders、分类、安装签名比较 |
| `src/v2/pages/mcp/constants.ts` | `DEFAULT_NEW_APPS` 单源，供已安装编辑器与发现安装共用 |
| `src/v2/pages/mcp/Discovery.tsx` | 搜索、分类、卡片状态、触发安装 |
| `src/v2/pages/mcp/InstallDialog.tsx` | schema 驱动业务字段 + 六目标多选 |
| `src/v2/pages/mcp/Page.tsx` | 页签协调；已安装行为保持；详情使用脱敏展示 |

## Data flow

1. 发现页读取静态 `MCP_CATALOG` 与 `useMcpServers()`。
2. 搜索只匹配 Catalog 公共元数据（名称、描述、标签、厂商、分类）。
3. 卡片用已安装 map 按 ID 判断状态，再用 `catalogSignature` 比较 type/command/非机密 args 前缀/URL origin+pathname/env 与 header 的键名。
4. 无字段：直接 `build({}, DEFAULT_NEW_APPS, platform)` → `upsert` → invalidate `featureKeys.mcp`。
5. 有字段：打开弹窗；校验通过后同样 upsert。
6. 同 ID 且签名不同：确认后才 upsert。关闭弹窗时清空 values。

## Contracts

### Catalog item

```ts
type McpCatalogCategory =
  | "china"
  | "devtools"
  | "collab"
  | "maps"
  | "multimodal"
  | "basics";

type McpProvenance = "official" | "reference" | "community";

interface McpCatalogItem {
  id: string;
  name: string;
  description: string;
  categories: readonly McpCatalogCategory[];
  tags: readonly string[];
  publisher: string;
  provenance: McpProvenance;
  homepage?: string;
  docs?: string;
  requirements: readonly ("none" | "node" | "uv")[];
  fields: readonly McpInstallField[];
  risk?: string;
  recommended?: boolean;
  build(
    values: Record<string, string | string[]>,
    apps: readonly McpTargetId[],
    platform: ReturnType<typeof detectNativePlatform>,
  ): McpServer;
}
```

`build` 在缺必填字段时抛出 `UserFacingError`，UI 在按钮层先禁用，测试覆盖抛错路径。

### Security

- `redactMcpUrl(url)`：敏感 query 值替换为 `••••••`。
- `redactMcpArgs(args)`：敏感 flag 后一项替换为 `••••••`。
- `buildMcpSearchText`：args 用脱敏后文本；url 只用 origin+pathname；永不加入 env/headers。
- 普通详情展示脱敏 URL/args；env/headers 仍只显示项数。

### Conflict

签名一致 → 主按钮「已安装」，可「查看」切到已安装页签。
ID 存在但签名不同 → 「已存在」+「重新配置」。
未安装 → 「安装」或「配置并安装」。

## Compatibility

- 已安装 CRUD、导入、手动模板入口保持。
- Context7：发现页走 `https://mcp.context7.com/mcp` HTTP；「添加 MCP」模板继续提供 npx 回退，发现卡片不展示第二套。
- Time / Memory / Fetch 的预设 command/args 必须与 Catalog builder 一致，用测试锁住。
- 不写 `source` 到 upsert 载荷，避免前端有值、后端丢字段。
- 不改 MCP target 集合。

## Trade-offs

- 静态 Catalog 换来零后端市场接口；名单更新需要发版。
- 无字段条目一键安装使用默认四目标，而不是每次弹窗选 Agent；用户仍可在已安装页改分配。
- unknown 平台改走 Unix npx，避免精选在未知 UA 上整卡消失；这不是 Linux 产品支持承诺。

## Rollback

删除页签/发现模块并恢复 `buildMcpSearchText` 与 presets 即可回退。已用发现页安装的 MCP 仍是普通 `McpServer` 行，可用现有删除流程清理。不涉及数据库迁移。
