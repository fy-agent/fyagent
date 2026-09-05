# 告警处置与残余风险（2026-09-05）

## 范围与身份

仓库 `fy-agent/fyagent`；本地基线 `b580bf90`，分支 `dev/laiyongjie`。
GitHub CodeQL 的 66 条 open 记录扫描的是 `5dcfb8ef6bc2be392e0b7669ddad63f4a62782b9`，不是本地工作树。
通过 `gh api` 分页获取，只保留编号、规则、位置、ref/SHA、严重性和诊断；未修改远端告警或安全开关。
下表“已修复”均指本地代码/锁图，不表示 GitHub 已关闭告警。

## Dependabot：51 条记录的完整包级映射

| 编号                                    | 包/依赖族               | 本地处置                                                                            |
| --------------------------------------- | ----------------------- | ----------------------------------------------------------------------------------- |
| 62                                      | nanoid                  | 定向更新锁图，最终 pnpm audit 不再命中；同时修复独立审计检出的后续公告。            |
| 60                                      | browserslist            | 更新锁图，保留当前 CSS 工具体系。                                                   |
| 59                                      | postcss-selector-parser | 更新兼容修复版本。                                                                  |
| 58                                      | serde_with              | 3.22.0，宏依赖随同更新。                                                            |
| 54、27、26、16                          | postcss                 | 直接依赖下限提高至修复范围，锁图 8.5.28。                                           |
| 53                                      | quinn-proto             | 0.11.17。                                                                           |
| 50、49、47、45、44、43、42、41          | openssl                 | 0.10.81，未另建 TLS 实现。                                                          |
| 48                                      | tauri                   | 2.11.1，修复 local-origin 判定；必要的同族依赖随 Cargo 解析更新。                   |
| 46、38、37、36                          | rustls-webpki           | 0.103.15。                                                                          |
| 35、34、31、30、29                      | aws-lc-sys              | 0.45.0，由现有依赖链升级获得。                                                      |
| 40、39                                  | rand 0.8 / 0.9          | 分别 0.8.8 / 0.9.5；另一个旧 0.7 构建节点的残余告警见下文。                         |
| 28                                      | glib                    | 保留上游 0.18.5 条件性告警；不能把 gtk 0.18 的依赖强行替换成不兼容 glib 0.20。      |
| 25                                      | form-data               | 更新兼容锁图节点。                                                                  |
| 24、23、22、21、15、14、13、12、5、4、3 | vite                    | 直接依赖保留 7.x，锁定 7.3.6；Vitest 更新消除旧开发服务器链，完整审计复核全部节点。 |
| 20                                      | @babel/core             | 更新兼容锁图节点。                                                                  |
| 19                                      | ws                      | 更新兼容锁图节点。                                                                  |
| 18、1                                   | vitest                  | 2.1.9 -> 3.2.7，不跃迁到最新大版本；严格检查测试迁移。                              |
| 11、10                                  | picomatch               | 更新各受影响兼容分支，不用跨 major override。                                       |
| 7                                       | smol-toml               | 锁图 1.8.0。                                                                        |
| 6                                       | rollup                  | 更新兼容修复节点。                                                                  |
| 2                                       | esbuild                 | 通过父依赖升级消除旧节点。                                                          |

独立 Rust 审计还发现 h2、quick-xml、rkyv 等问题。h2 更新到 0.4.19；移除没有源码消费者的直接 quick-xml 0.38，升级仍使用 XML 的 plist 链到 quick-xml 0.41；rust_decimal 更新到 1.43 后旧 rkyv 节点被删除。serial_test 3.5 消除旧 scc，uds_windows 更新到 1.2.1 消除 yanked 节点。没有关闭 unsafe、修改协议或强行替换传递依赖的 API。

最终独立审计：pnpm 所有严重性均为 0；cargo-audit vulnerability=0，unmaintained=17、unsound=2，yanked=0。RustSec DB 为 `5a0ebedfe8bdd2e295b171f4162f8c977bcad9a5`（1239 条公告）。这是该数据库快照下的已知问题检查，不是未知漏洞证明。

## CodeQL：66 条记录逐组处置

| 编号       | 证据和判断                                                                                                     | 处置/限制                                                                                                                                                                      |
| ---------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1          | Release preflight 的显式候选 SHA 可不同于可信 main workflow SHA；默认分支执行上下文具有跨工作流缓存风险。      | 移除 Release 的 pnpm/Cargo cache 消费，并显式关闭 setup-node 自动缓存。属于缓解，不是完整候选代码沙箱；不宣称该告警已彻底消除。                                                |
| 2          | 旧扫描指向 task runner 的 shell-string 路径；本地基线已使用 spawnSync(argv)。                                  | 本轮显式 `shell:false`，保持受控 argv 和既有参数回归。                                                                                                                         |
| 3          | 旧 hdiutil 测试动态 shell；本地已改为固定 BASH_RUNNER + 独立位置参数。                                         | 保留现有包含空格/引号的参数回归，不删测试。                                                                                                                                    |
| 4、5       | 平台扫描器 RegExp 模板中的 `\s` 被 JS 字符串消费。                                                             | 修正双层转义；新增带空白 switch selector 的反例。                                                                                                                              |
| 6–9        | ProviderForm 四处 URL 子串识别可被 path/query/相似域绕过。                                                     | 同一 `isCopilotEndpoint`，标准 URL + HTTPS + 无 userinfo + 精确域/点分子域；仅 UI 身份提示，不授予凭证路由权限。                                                               |
| 10         | 原有 forbidden-key 检查存在，但继承对象仍可能被递归修改、继承 setter 可被触发。                                | own-property 检查与 defineProperty 写入，保留覆盖数组及就地 merge 语义；补污染和继承 setter 回归。                                                                             |
| 11、12     | 深链接检查页把用户参数拼成 innerHTML。                                                                         | 移除动态 innerHTML，改原生 DOM textContent/replaceChildren；恶意标记、JSON、脚本只呈现为文本。                                                                                 |
| 13、14     | 旧深链接生成器把 DOM 文本重新拼成 HTML；本地基线生成链接已经使用 DOM 属性。                                    | 核对当前调用链，生成链接仍是固定 fyagent 协议和编码参数，未复活旧 sink。                                                                                                       |
| 15、17、18 | 构建预览用正则匹配标签和删除脚本，不能等价 HTML 解析。                                                         | 复用 parse5 的元素/属性/位置，删除特定 file-redirect 标记元素；评论、template、raw-text、实体和畸形 end tag 回归。                                                             |
| 16         | release-contract 去注释仅用于判断 CHANGELOG 本版是否非空，不把清理后结果送到 HTML sink。                       | 保留局部非空校验；明确它不是 HTML sanitizer，不以此做渲染安全保证。                                                                                                            |
| 19         | Markdown cell 只转义 pipe，没有先转义反斜杠。                                                                  | 先转义反斜杠，补连续反斜杠/pipe 回归。                                                                                                                                         |
| 20         | Host PackageBridge 在读完整 ACCESS_ALLOWED_ACE 后才检查记录长度。                                              | 先检查共用 header、SID 最小边界，再创建完整 ACE 引用；掩码/所有者验证不放宽。仅源码顺序测试与当前宿主回归，仍需 Windows 原生证据。                                             |
| 21、22     | helper 对 OS 返回、由仍存活 descriptor 持有的 ACE 取引用；现有 header/type/minimum-size/SID 长度检查已经在前。 | 保留经检查的 GetAce/descriptor 生命周期，不绕开类型/ACL。没有 Windows 真机/Miri 证明，不能直接标为已由本轮修复。                                                               |
| 23–31      | 对照旧扫描 SHA，均位于 cfg(test) 的合成凭证、错误或 redaction 断言。                                           | 保留测试；不把测试 assertion/panic 的夹具输出等同于生产秘密日志，也不删断言来降告警。                                                                                          |
| 32–36      | 五个 session provider 的合成 hostile-ID 测试断言。                                                             | 保留防命令注入测试；不是生产认证 token 输出。                                                                                                                                  |
| 37、38     | 旧 forwarder 会打印指定 account/credential ID；本地基线 managed-auth 路径已移除相关诊断。                      | 核对当前所有分支，不重复移植历史修复。                                                                                                                                         |
| 39、40     | handler_context 打印客户端提供的 session ID。                                                                  | 移除 ID；保留来源、是否客户端提供及 provider 数等非秘密信息。                                                                                                                  |
| 41         | TLS root store 的 added 是证书数量。                                                                           | 保留数量，不输出证书或密钥内容。                                                                                                                                               |
| 42、43     | Codex OAuth 刷新/移除输出账号 ID。                                                                             | 改固定操作事件，认证流程不变。                                                                                                                                                 |
| 44         | OAuth account map 的 len。                                                                                     | 只打印数量，不是账户值或凭证。                                                                                                                                                 |
| 45–54      | Copilot 操作输出账号、企业域或原始失败信息。                                                                   | 改固定操作/计数/到期信息，去掉不必要标识和错误插值。                                                                                                                           |
| 55         | Copilot 保存日志仅输出 map.len。                                                                               | 保留数量。                                                                                                                                                                     |
| 56–62      | Gemini/Grok/OpenCode 同步日志拼入 session/request ID 或包含该 ID 的路径/错误。                                 | 普通日志改固定事件；Grok deferred warning 去掉 ID/任意模型字符串。保持数据库业务 ID 和显式同步结果不变，不宣传全应用诊断已全面脱敏。                                           |
| 63、64     | WebDAV auth_from_credentials 的 cfg(test) 固定假密码。                                                         | 保留空用户名/合法凭据边界测试。                                                                                                                                                |
| 65         | `[0;32]` 是 BCryptGenRandom 输出缓冲区，失败状态会返回错误，成功后才编码 nonce。                               | 保留 OS CSPRNG 和返回值检查；零初始化本身不是硬编码 nonce。                                                                                                                    |
| 66         | Required 0.1.1 的 evaluateCertificateEqualsHash 实现 SHA1 证书哈希比较。                                       | 保留上游语义；项目显式 helper requirement 使用 identifier + certificate leaf OU，非该哈希规则。未证明整个上游调用图不可达，不替换 Apple requirement 语义、不宣称消除上游告警。 |

没有运行新的完整 CodeQL 分析，也没有上传 SARIF/关闭告警。上述结论是旧扫描记录与当前源码及专项测试的对照，后续远端扫描可能保留条件性或第三方记录。

## Secret scanning 与独立 Gitleaks

GitHub #1/#2 对应历史 `subscription.rs` 的 Gemini OAuth client ID/secret。用官方 `google-gemini/gemini-cli` commit `f743ab579098f982d87ea3f2472c2405f6999297` 的 `packages/core/src/code_assist/oauth2.ts` 做内存内比较，两常量均匹配（只记录 boolean，不保存值或哈希）。这是公开 installed-app client 标识，不是用户 access/refresh token；不撤销第三方 OAuth client，也不擅自关闭告警。

Gitleaks 8.30.1：初始 HEAD archive 17 个候选；修改后当前 tracked + 非忽略 untracked 文件快照扫描 2946 个常规文件，15 个候选，exit=1（发现匹配，非工具失败）。剩余 6 个 HTML 文档候选是编码的演示配置，9 个是 Codex/Gemini/V2 测试输入。保留原始路径/行对应关系与语义测试；未加 allowlist 或把“示例”一概当作安全凭证。原始/编码 Context7 凭证样式值已经替换为显式占位符并测试编码回读。

此前历史扫描涵盖本地全部 refs 的 2769 个提交，32 个候选；未重写历史。Context7 历史值真实性与归属无法从源码证实，也没有用它请求服务。**若曾是真实凭证，所有者仍必须撤销/轮换；当前源码删除不等于凭证失效。**

## 保留问题与后续触发条件

1. `glib 0.18.5` / GTK 0.18 family：锁图仍有上游 unmaintained/unsound。当前 macOS ARM64 和 Windows x64 Cargo 反向图不包含 glib；其他目标证据不能从这两次元数据查询推断。未来 Tauri/GTK 升级或开发宿主改变时重新评估，不做不兼容 override。
2. `rand 0.7.3`：来自 `tauri-utils -> kuchikiki -> selectors -> phf_codegen -> phf_generator` 构建链；检查到的 macOS feature 图不启用公告必需的 log feature。未来 feature/构建链变化需重审；无 0.7 同线补丁时不强行升级到 0.8。
3. 17 条维护性警告的逐包清单保留在 cargo-audit 输出：atk/atk-sys、fxhash、gdk/gdk-sys/gdkwayland-sys/gdkx11/gdkx11-sys、gtk/gtk-sys/gtk3-macros、proc-macro-error、unic-char-property/unic-char-range/unic-common/unic-ucd-ident/unic-ucd-version。
4. Release preflight 不是任意不可信候选的隔离沙箱。GitHub 手动 dispatch 需要仓库写权限；仍应只选择已审查候选。不为了清除一个静态告警而静默取消现有 preflight 的用途。接入不可信自动 dispatch 前必须另行建立执行/缓存/签名信任边界。
5. Windows 原生 ACL、最低 macOS WebView、实际系统凭证存储、真实云端、签名/发布没有在本轮执行；浏览器与 macOS portable tests 不替代它们。

## 官方依据

- Tauri advisory：https://github.com/tauri-apps/tauri/security/advisories/GHSA-7gmj-67g7-phm9
- parse5：https://parse5.js.org/ 。它负责 HTML 语法，不充当任意不可信构建产物的 sanitizer。
- dependency-cruiser：https://github.com/sverweij/dependency-cruiser
- Google installed apps：https://developers.google.com/identity/protocols/oauth2/native-app
- Google 源码：https://github.com/google-gemini/gemini-cli/blob/f743ab579098f982d87ea3f2472c2405f6999297/packages/core/src/code_assist/oauth2.ts
- GitHub secret remediation：https://docs.github.com/en/code-security/tutorials/remediate-leaked-secrets/remediating-a-leaked-secret
- Windows GetAce：https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-getace
- RustSec：https://rustsec.org/advisories/RUSTSEC-2024-0429.html 、https://rustsec.org/advisories/RUSTSEC-2026-0097.html
- 手动工作流权限：https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow

Required 的远端网页读取失败；该项证据使用 Package.resolved 中的 pinned revision `82a4fbd388346ca40b1bbe815014dc45a75d503c`、本地 checkout 源码和项目 XPCSession requirement，不能假装完成了额外的在线安全认证。
