# 第二轮集成评审

## 实现与责任边界

- 修复依赖而不是关闭扫描：Tauri 保留 2.x、Vite 保留 7.x，Vitest 升至有修复的 3.2；完整锁图重审而非仅查看 production npm 包。
- HTML 标准语法交给 parse5；项目只保留资产路径、内联与明确构建标记的策略。不把 parser 当作 sanitizer，不对任意不可信 HTML 提供执行安全承诺。
- 原生 DOM 文本 API 替换深链接检查页的动态 HTML 拼接；原始配置仍保留明确的本地展开查看入口，因此不能宣传所有凭证字段永不进入页面。
- 四处 Provider URL 识别共用一个标准 URL 谓词；配置结构操作不修改继承对象或触发继承 setter。
- Feature-aware controls 与纯 UI 分离，删除未使用的 CatalogOfficialLinks；dependency-cruiser 进入根单元测试入口，真实 TypeScript 环境下检查运行时图。
- 原生 CSPRNG、IPC/ACL、数据库 schema、补偿和同步业务逻辑不更换。日志仅调整诊断输出；PackageBridge 前置 header/size 检查，原有权限掩码和 SID 校验仍保留。
- Release 不再复用共享 pnpm/Cargo 缓存；既有 candidate 执行的信任限制如实保留于风险记录，不以缓存删除冒充沙箱。

## 三轮复核

1. 远端告警与独立扫描建立基线；初次缺少 TypeScript 的临时图扫描被废弃。
2. 按实际源/汇/生命周期修复，比较升级范围与现有 owner，保持父模块和 UI/原生边界。
3. 对所有 diff、旧扫描 SHA、当前声明/测试、凭证样例编码及 SPEC 引用重审；新增断言发现遗漏后继续修复，不删除失败断言。

这三轮是同一执行者的复核，不是独立外部安全审计。

## 发现的集成问题及处理

- 最终 CI 差异审查发现新 parser/声明/依赖图配置未被 classifier 识别，架构测试单改也可能仅触发较小的 contracts 子集。补齐精确归属及逐文件调度回归，unknown-path fail-closed 和 Full CI 权限保持原样。

- Vitest 3 的 SVG 测试加载变化：显式设置 V2 测试 assetsInlineLimit=0，保持真实本地资产断言，不放宽为任意 URL。
- 真实签名校验脚本测试原先超过默认 5 秒；对两项既有子进程集成测试给出局部 30 秒上限，全部信任拒绝断言保持不变，没有全局提高测试时限。
- 新增脚本补 `.d.mts` 公共契约；补 `@types/jsdom` 并删除因此失效的旧 ts-expect-error。修正测试中的返回值联合分支，不用隐式 any 掩盖不兼容。
- Object.hasOwn 改成已有运行库支持的 hasOwnProperty.call，不提高应用目标环境。
- 全量门禁发现 Grok 定价漂移的另一条 deferred warning 仍含 ID，补齐固定诊断；ACL 既有测试同步前置 header 字段写法，专门测试继续验证读取顺序。
- 平台 identity seal 只更新已审查的 11 个既有文件摘要（runner、checker、Cargo、两 OAuth 模块、Grok sync、subscription、Release/平台/helper/preview 测试）；不新增 exclusion、修改权限或降低检查阈值。

## 验证记录

| 检查                                    | 可核验结果                                                                                                      |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| 根 TypeScript / V2 TypeScript / V2 lint | 通过；类型声明补全后复跑                                                                                        |
| V2 单元测试                             | 72 文件，512 passed                                                                                             |
| 浏览器回归与 renderer build             | 164 passed；生产构建、路由 chunk 校验和 standalone preview 成功                                                 |
| 根前端单元测试                          | 183 文件，1618 passed / 1 skipped；最终完整门禁结果                                                             |
| Rust check/Clippy/fmt/完整 tests        | 本机 macOS ARM64 完整门禁通过；3469 passed / 0 failed / 6 ignored                                               |
| 运行时依赖图                            | 736 模块、2846 条依赖，0 error / 0 warn；TS 环境无缺失警告                                                      |
| pnpm audit                              | 0 漏洞，退出 0                                                                                                  |
| cargo-audit                             | vulnerability 0，17 unmaintained + 2 unsound；退出 0 不等于警告消失                                             |
| Gitleaks 修改后工作树快照               | 2946 常规文件、15 候选、退出 1；保留匹配及分类                                                                  |
| Gitleaks 基线全 refs 历史               | 2769 commits、32 候选；无历史重写                                                                               |
| GitHub 告警                             | 51 Dependabot / 66 CodeQL / 2 Secret scanning，全部在 alert-disposition.md 建立处置映射；没有关闭或远端复扫声明 |
| 最终完整 prearchive                     | 通过，exit=0；最新 CI 分类及全部源码修复均已包含；release-contract 子集 611 passed / 1 skipped                  |
| 归档后无排除 contracts                  | 归档后执行并记录                                                                                                |

本机临时日志位于 `/tmp/fyagent-round2-*`，它们不是项目运行依赖。长期结论保留在本任务文件，不依赖临时文件仍在。

## 残余与后续

详见 `research/alert-disposition.md`。特别是 glib/rand 上游条件、Required SHA1 兼容分支、Release preflight 信任隔离、Context7 历史值由所有者确认并按需轮换，以及 Windows/最低系统/凭证/真实云端/签名原生证据。不得将本任务 completed 状态解释为这些项目均已消失。
