# FyAgent 跨平台控制面高保真原型 v3

## 评审入口

| 页码 | 一级页面 | 原型文件 | 主要覆盖 |
| --- | --- | --- | --- |
| 01 | Agent 目录 | [01-agent-catalog-v3.png](./01-agent-catalog-v3.png) | 五个受控候选、能力概览、接入检查器 |
| 02 | 模型 | [02-models-v3.png](./02-models-v3.png) | 接入源、默认/备用路由、代理、故障切换 |
| 03 | Skills | [03-skills-v3.png](./03-skills-v3.png) | 已安装、发现、来源、备份、Agent 分配 |
| 04 | MCP | [04-mcp-v3.png](./04-mcp-v3.png) | 服务、传输方式、权限、Agent 分配、连接检查 |
| 05 | 提示词 | [05-prompts-v3.png](./05-prompts-v3.png) | 提示词库、编辑器、Agent 分配、覆盖优先级 |
| 06 | 记忆 | [06-memory-v3.png](./06-memory-v3.png) | 长期记忆、每日记录、身份与偏好、可见范围 |

## 本轮解决的问题

- 删除 v2 的口号式副标题、“查看来源”、底部说明条和重复的系统能力快捷卡。
- 五个 Agent 从等宽卡片改为连续列表，选中后在中间工作区配置。
- 六个一级导航在所有页面保持相同位置、顺序和选中逻辑。
- 采用 Windows 右上角窗口控件作为本轮壳层示意；macOS 不复制红黄绿按钮，交由原生标题栏承载。
- 模型、Skills、MCP、提示词、记忆都采用“左侧对象 / 中间工作 / 右侧范围或状态”的同一交互语法。

## 输出验证

- 生成工具：Codex 内置 `image_gen`。
- 母版参考：`fyagent-control-plane-prototype-v2.png`。
- 生成器将 1440×900 的 16:10 设计意图标准化输出为 1586×992；六张尺寸和比例完全一致。
- 证据等级：`generated_prototype`，不是 `runtime_screenshot`，不代表功能已经实现。
- `git diff -- src assets/fyagent.png` 无输出；本轮没有修改产品前端或品牌主文件。

## 文件校验

| 文件 | SHA-256 |
| --- | --- |
| 01-agent-catalog-v3.png | `7EE5EE73ADAC394DDC93DE59534EDE0884FFB4B1C8DBAB0F774788E1A25FBD7E` |
| 02-models-v3.png | `7770C6EBFBA8EE8F36A601F2E5F412CB7B50247F5286223239DB74D53C62F665` |
| 03-skills-v3.png | `4EC69E778BC158696672FE86866AA4849CE7D81F05DEBE29324B5EBFC69109EE` |
| 04-mcp-v3.png | `4C876FD190B3148537A31F25AD38AABA1A16FD708E3DC24CBD49282402994B9E` |
| 05-prompts-v3.png | `47D5B2A19DBC05BD90A888C09F533881C88282889B052085DC7FEC576AE481E8` |
| 06-memory-v3.png | `B1C9DE93A0E716AFB661AB632DBD655AA8FFFD288CD09B447902BE1F41F76E33` |

## 相关依据

- [现有前端能力盘点](./frontend-capability-inventory.md)
- [六页页面规范](./control-plane-v3-page-spec.md)
- [v3 Design DNA](./control-plane-v3-design-dna.json)
- [生成提示词](./control-plane-v3-generation-prompts.md)
