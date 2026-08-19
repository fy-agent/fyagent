# V2 Skills 发现分页 — 技术设计

## Architecture and boundaries

```text
pages/skills/Page.tsx Discovery
  -> shared/ui FeaturePagination, FeatureSearch, FeatureTabs
  -> shared/features types, ports, queries
  -> shared/platform/tauri invoke discover_available_skills_page
  -> commands/skill.rs
  -> SkillService cached scan + filter + slice
```

MCP 发现只改滚动 class，不改数据源。leftover `src/lib/api/skills.ts` 继续走 `discover_available_skills`。

## Why not GitHub Trees API

仓库搜索需要 `SKILL.md` front matter 里的 description。Trees API 只给路径，仍要按文件拉元数据；未认证 GitHub API 还有限流。现有 zip 下载/解压预算已经过安全审查。因此：扫描仍走 zip，结果缓存在进程内，分页只约束 IPC 与渲染。

## Signatures

```ts
const SKILL_DISCOVERY_PAGE_SIZE = 20;
const SKILL_DISCOVERY_MAX_PAGE_SIZE = 50;

type SkillDiscoveryStatus = "all" | "installed" | "uninstalled";

interface DiscoverableSkillsPage {
  skills: DiscoverableSkill[];
  totalCount: number;
}

interface DiscoverSkillsPageRequest {
  query: string;
  repo?: string;
  status: SkillDiscoveryStatus;
  limit: number;
  offset: number;
}

interface SkillsPort {
  discoverPage(request: DiscoverSkillsPageRequest): Promise<DiscoverableSkillsPage>;
  // leftover-only discover_available_skills is not on the V2 port
}
```

```rust
discover_available_skills_page(
  query: String,
  repo: Option<String>,
  status: String, // all | installed | uninstalled
  limit: usize,
  offset: usize,
) -> DiscoverableSkillsPage { skills, totalCount }
```

Invalid `status` → command error. `limit == 0` → 20；`limit > 50` → 50。`offset` 超过总数 → `skills: []`，`totalCount` 仍为过滤后总数。

## Data flow

1. Discovery 进入仓库来源时，`useSkillDiscoveryPage(debouncedQuery, repo, status, page)`。
2. 宿主读取启用仓库；缓存命中则跳过 zip。未命中则 `discover_available` 扫描并写入缓存（fingerprint = 排序后的 `owner/name/branch`，TTL 5 分钟）。
3. 按 query（name/description/`owner/name`）、repo（`owner/name`）、status（与前端 `isDiscoverableInstalled` 相同：directory tail + owner/name）过滤，再 `skip(offset).take(limit)`。
4. 增删仓库命令失效缓存。
5. 写操作仍 `invalidateQueries({ queryKey: featureKeys.skillDiscovery })`，前缀匹配各页。

## Scroll

`.fy-feature-discovery-scroll` 放在 `features.css`：

- `flex: 1 1 auto; min-height: 0; overflow: auto;`
- `overscroll-behavior: contain; scrollbar-width: thin;`

MCP `Discovery.tsx` 用该类替换 `.fy-mcp-discovery`。Skills 结果卡片用该类；工具条与 `FeaturePagination` 是 workspace 的非滚动子项。

## Shared chrome

`FeaturePagination` 进入 `src/v2/shared/ui`（仓库与 skills.sh 两个来源，且 MCP 之后也可能用）。页码窗口保持现有 `page-3 .. page+2` 切片。

## Compatibility / rollback

- 新命令加入 `lib.rs` 与 `legacy-application-commands.toml`。
- 回滚：恢复 V2 port 的 `discover()` 全量调用，并删除新命令；zip 扫描路径不变。
