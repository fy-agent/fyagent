# V2 Skills 发现分页与共享滚动

## Goal

V2 Skills「发现」页浏览仓库 Skills 时，不再一次性把全部条目拉到渲染层。卡片区使用与 MCP 发现相同的独立滚动容器；仓库来源按页向宿主请求，搜索/筛选在分页之前完成。

## Confirmed facts

- Skills 发现有两个来源：仓库（`discover_available_skills` 一次返回全量 `DiscoverableSkill[]`）和 skills.sh（已有 `limit`/`offset`/`totalCount`）。
- 仓库发现当前会下载 GitHub zip 并扫描全部 `SKILL.md`。搜索、仓库筛选、安装状态筛选都在前端对全量数组做。
- MCP 发现用 `.fy-mcp-discovery`（`overflow: auto`、`scrollbar-width: thin`、`overscroll-behavior: contain`）。Skills 发现误用 `.fy-feature-detail-scroll`，该类没有 overflow，本属于分栏详情轨。
- leftover V1 仍调用 `discover_available_skills` 全量 API，不能改返回类型。
- 仓库筛选 tab 目前从全量发现结果推导；已配置仓库来自 `get_skill_repos`。

## Requirements

### 滚动

- R1. 把 MCP 发现的滚动配方提取为共享 `.fy-feature-discovery-scroll`，放进 `features.css`。
- R2. Skills 发现卡片区与 MCP 发现都使用该类。不要给 `.fy-feature-detail-scroll` 加 overflow。
- R3. Skills 发现的搜索/筛选工具条和分页条留在滚动区外；只有结果卡片滚动。

### 仓库分页获取

- R4. V2 不得再调用全量 `discover()`。新增分页命令，每页默认 20 条，limit 上限 50。
- R5. 响应为 `{ skills, totalCount }`。渲染层只绘制当前页。
- R6. 搜索、仓库筛选、安装状态筛选在宿主侧、slice 之前完成。改筛选时回到第 1 页。
- R7. 仓库筛选选项来自已启用的 `getRepos()`，不依赖全量 Skill 列表。
- R8. 仓库扫描结果在 `SkillService` 内按启用仓库指纹缓存；增删仓库必须失效缓存。缓存未命中时仍可扫描整仓（现有 zip 路径），但 IPC 只返回当前页。
- R9. leftover `discover_available_skills` 保持全量返回，供 V1 使用。
- R10. skills.sh 继续使用现有分页；与仓库分页共用数字分页控件。

### 非目标

- 不改 GitHub zip 下载/解压安全边界，不引入 GitHub Trees/Search API。
- 不改 MCP 精选静态目录、已安装 Skills 主从布局、Agents/Models/Prompts/Memory。
- 不为 leftover Skills 页做分页。

## Acceptance criteria

- [ ] AC1. Skills 发现与 MCP 发现都使用 `.fy-feature-discovery-scroll`；Skills 结果区可独立滚动，工具条和分页不随卡片滚走。
- [ ] AC2. 仓库发现每页最多 20 条；`totalCount > 20` 时出现与 skills.sh 同类的数字分页；翻页会带着 `limit`/`offset` 再请求。
- [ ] AC3. 搜索/仓库/安装状态筛选在服务端过滤后分页；改筛选重置到第 1 页。
- [ ] AC4. 浏览器预览的 `discoverPage` 读返回空页，不发起全量发现。
- [ ] AC5. leftover `discover_available_skills` 签名与返回 `Vec<DiscoverableSkill>` 不变。
- [ ] AC6. 组件测试覆盖：分页请求、筛选重置页码、已安装匹配仍按 directory tail + owner/name。
- [ ] AC7. Rust 单测覆盖：limit clamp、offset 越界、query/repo/status 过滤、安装匹配。
- [ ] AC8. `mise run lint:v2`、`mise run typecheck:v2`、`mise run test:v2`、`mise run rust:fmt:check`、`mise run rust:test`（聚焦 skill）、`mise run format:check` 通过。

## Out of scope

- GitHub API 替代 zip 扫描。
- leftover V1 Skills 发现分页。
- MCP 精选远程市场或 MCP 发现分页。
- 已安装 Skills 列表虚拟化。
