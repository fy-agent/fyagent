# FyAgent #377：omp 真实 provider 配置技术输入

核查日期：2026-10-22。证据层级：**official_docs + code_audit**；本轮未安装、启动 omp，未读取用户凭据、配置或数据库，未修改项目源码。以下以稳定版 **v18.12.5 / `d5cd750e41fede277f9eb933455da92a0738915e`** 为准，官网滚动文档和搜索缓存存在细节差异。

## 产品、版本与 Windows

omp 是 Stencil Labs 的 Oh My Pi，官方仓库为 [`can1357/oh-my-pi`](https://github.com/can1357/oh-my-pi)，官网为 [omp.sh](https://omp.sh/)，包名 `@oh-my-pi/pi-coding-agent`。当前稳定版 v18.12.5 于 2026-10-21 02:31:29 UTC 发布，含 `omp-windows-x64.exe` 和 `omp-windows-arm64.exe`；Windows x64 使用官方原生发行目标。[发布记录](https://github.com/can1357/oh-my-pi/releases/tag/v18.12.5)

官方 Windows 安装入口是 `irm https://omp.sh/install.ps1 | iex`，也支持 Bun 全局安装；脚本要求 PowerShell ≥ 5.1，Bun 安装方式要求 Bun ≥ 1.3.14。默认有 Bun 时走 Bun，否则取预编译二进制；可用 `-Binary -Ref v18.12.5` 固定二进制版本。二进制默认目录为 `%LOCALAPPDATA%\omp`，支持 `PI_INSTALL_DIR`，下载后核对 SHA-256。**本次仅核实发行和安装代码，尚无本机安装或运行验收。** [固定版安装脚本](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/scripts/install.ps1#L16-L55) · [二进制校验](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/scripts/install.ps1#L286-L345)

## 实际配置边界

| 内容 | 默认 Windows 位置 / 规则 |
| --- | --- |
| Provider、endpoint、API key 引用、模型元数据 | `%USERPROFILE%\.omp\agent\models.yml`；不存在时读同目录 `models.yaml`，两者不合并 |
| 模型角色和用户设置 | 同目录 `config.yml`，兼容已有 `config.yaml`；默认模型用 `modelRoles.default: provider/model-id` |
| 项目设置 | `<cwd>\.omp\config.yml`；可影响模型选择、禁用项，但不等于另一个项目 models.yml |
| 原生登录数据 | 默认同目录 `agent.db`；包含其他状态，不能为接入 provider 清空或改写 |
| 活动 profile | `--profile` 优先于 `OMP_PROFILE`，后者优先于兼容的 `PI_PROFILE`；命名 profile 位于 `~/.omp/profiles/<name>/agent` |
| 目录重定位 | `PI_CONFIG_DIR` 改 config-root 名称；`PI_CODING_AGENT_DIR` 仅覆盖 default profile 的 agent 目录；命名 profile 忽略该覆盖 |

路径必须由活动环境解析；后续可用 `omp config path` 或 `omp --profile <name> config path`核对。Windows 使用上述用户目录规则。仅存在 `models.json` 时，omp 自己会迁移到 YAML；FyAgent 需要识别、保全旧内容并展示迁移预览，不先写一份空 YAML 抢占旧配置。依据：[目录实现](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/utils/src/dirs.ts#L328-L405) · [profile 与目录规则](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/settings.md#L13-L28) · [YAML 选择和旧 JSON 迁移](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/config/config-file.ts#L166-L194)

本仓现有 `AppId 'pi'` / `services/provider/pi.rs` 使用 Pi 的 `models.json`。**omp 使用独立软件目录身份、模型 target、目录解析和 YAML 读写服务；复用 provider 合并、原子写、CAS 的结构理念，保持 Pi 的路径和 JSON 序列化不变。**本次通过独立模型端口接入，既有 MCP、Skills、Prompts 的应用枚举仍保持原有 7 项。

## Provider 格式、协议与认证

最小 OpenAI-compatible 配置示例（占位值，未用于真实请求）：

```yaml
providers:
  fyagent-example:
    baseUrl: https://gateway.example.com/v1
    api: openai-completions
    apiKey: FYAGENT_OMP_EXAMPLE_KEY
    auth: apiKey
    models:
      - id: model-from-provider
        name: Example model
```

`api: openai-completions` 对应 Chat Completions；Responses 服务应明确选择 `openai-responses`。`openai-codex-responses` 是另一种传输，不能由“OpenAI-compatible”自动推断。`baseUrl` 保留服务实际根路径，不盲目拼接 `/v1`。自定义模型必须有 provider `baseUrl`、`apiKey`（`auth: none/oauth` 除外），以及 provider 或每模型上的 `api`。`contextWindow` / `maxTokens` 可选，若写入必须为正值；没有权威值就不虚构。无鉴权服务支持 `auth: none`；`auth: oauth` 本身不会注册登录流程或获得凭据。[校验实现](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/config/models-config.ts#L44-L132) · [协议与 auth 定义](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/models.md#L133-L218)

认证命名空间就是 provider ID。建议给 FyAgent 管理项使用稳定、独立的 `fyagent-<id>`，保持原生 `openai`、`openai-codex` 等既有登录独立；修改已有用户 provider 时按明确目标做局部合并。稳定版优先级为 runtime key → models.yml key → OAuth → `/login` key → extension fallback → provider env → 其他存储 key。models.yml 的 key 会压过该 provider 的 OAuth；不要为了接入自定义网关覆盖内置 provider ID 或复制其 OAuth token。[认证实现](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/ai/src/auth/cascade.ts#L210-L231) · [注册配置 key](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/config/model-registry.ts#L1838-L1857)

`apiKey` 字符串先按环境变量名解析；变量未设或空时会把字符串本身当作 token。仅写 `FYAGENT_OMP_EXAMPLE_KEY` 而未实际注入，不能算配置成功。支持 `!command` 动态取密钥，读取或保存配置时应保留该引用，不能由 FyAgent擅自执行。`authHeader: true` 会额外注入 Bearer 头，已有 headers/认证方式需要原样保留，新增时按真实 endpoint 决定。配置预览、日志及差异应脱敏；普通 provider 接入无需访问 `agent.db` 或 broker。[key 解析源码](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/ai/src/auth/cascade.ts#L23-L28) · [认证和命令引用规则](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/models.md#L583-L625) · [凭据存储](https://omp.sh/docs/secrets)

## 合并与用户字段保留

omp 以 `provider + model id` 识别模型；自定义模型在 catalog/discovery 后应用，匹配时更新定义字段，否则追加。provider headers 是基础，模型同名 header 优先；compat 的嵌套路由对象深合并。设置层也是对象深合并、数组整体替换，因此不能顺手重置 `disabledProviders`、`enabledModels`、其他角色或项目设置。[模型合成顺序](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/models.md#L298-L351) · [设置合并](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/settings.md#L162-L169)

FyAgent 的 YAML 编辑应以原文文档树为基础，局部更新选定 provider/模型，并保留：其他 providers；目标 provider 未被本次明确更改的 `headers`、`auth`、`authHeader`、`apiKey` 引用、`discovery`、`compat`、`modelOverrides`、`remoteCompaction`、`transport` 和其他字段；其他模型及目标模型的 reasoning/thinking/input/cost/limits/header/compat 元数据；根级未知字段、注释和可保留的 YAML 结构。稳定版 schema 不拒绝全部未知字段，未知 compat 字段仅警告，所以“只序列化已知字段”会破坏用户状态。[schema](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/config/models-config-schema-bundle.ts#L354-L402) · [未知 compat 保留逻辑](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/config/models-config.ts#L152-L194)

适配接口建议：`discover/status → readProviderConfig → previewSavePlan → apply(expectedRevision) → readback`。快照包含活动 profile、真实目标路径、文件 revision/hash、provider/model 身份及脱敏字段；apply 先核对最新文件、保留回退副本，再原子写入；冲突要求重新预览，不覆盖外部编辑。`.yaml` 已存在时继续写该文件；两文件并存时识别 `.yml` 为有效层并提示。YAML 无法解析时返回原文件位置和具体错误，不重建空配置。设置默认模型是独立、明确的写入意图，只更新 `modelRoles.default`。

## 接线与验收

本期完成链：独立 omp 识别与目录入口 → provider/model 列表和配置表单 → YAML 读取/局部合并 → 预览与保存 → 回读显示所选 provider/model → 安装检测与启动提示。安装卡片只是入口，不能替代这条链。

建议定向用例：新文件；已有 `.yaml`；两 YAML 并存优先级；旧 JSON 保全；带多个 provider 与未知字段的局部修改；带斜杠模型 ID；自定义 profile/目录；无效 YAML；CAS 冲突；连续保存幂等；写入失败回退；模型角色设置不清空其他角色；凭据不出现在预览/日志。测试使用虚构 key 和隔离目录。

后续 Windows 验证命令（本轮均未执行）：

```powershell
omp --version
omp config path
omp models find fyagent-example --json --no-extensions
omp config get modelRoles
```

模型清单会使用缓存，必要时可能联网；它不是纯静态验证。必须同时核对 stderr 无配置错误、JSON 中目标 `provider/id/selector` 精确存在，以及保存后文件回读一致。`models` 在配置错误时仍可能输出 JSON 且没有失败退出码，不能仅凭 exit 0 判通过；清单 JSON 不含 transport `api`，协议需另验持久文件。[models 命令](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/commands/models.ts#L13-L54) · [错误输出行为](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/cli/models-cli.ts#L227-L250) · [刷新和认证加载](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/packages/coding-agent/src/cli/models-cli.ts#L396-L424)

真实服务验收另做一次指定 `--model fyagent-example/<真实模型>` 的限时请求，使用已授权且实际注入的凭据；隔离工作目录，关闭工具、扩展和 session 持久化，并记录解析到的模型、响应和退出状态。只列出模型不能证明凭据或 endpoint 可用。可用参数包括 `-p --no-tools --no-extensions --no-session --no-title --max-time 60`；具体模型和请求由主流程在真实配置完成后确定。[CLI 参数](https://github.com/can1357/oh-my-pi/blob/d5cd750e41fede277f9eb933455da92a0738915e/docs/cli-reference.md#L67-L178)

交付状态应分别报告 `code_audit`、隔离配置测试、Windows CLI 解析、真实 provider 请求、FyAgent `runtime_screenshot`。本报告只完成前者及官方资料核实，不把源码支持写成本机已通过。
