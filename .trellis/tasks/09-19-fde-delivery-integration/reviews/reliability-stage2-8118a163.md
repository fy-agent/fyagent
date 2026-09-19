# 配置线阶段 2：R5 发布流程独立审查

范围：固定提交 `8118a163` 相对 `b0823673`，仅 R5。依据全部通过固定 `git show` / `git diff` 读取，包括工作流、调用脚本与已有测试。未读取漂移中的产品代码，未运行真实签名、公证、发布或全库测试。

## 结论

**有 1 个 P1 发布阻断，当前阶段不能通过。** 挂载 app 的目标和完整验证调用改对了，但 app 票据装订被移到了任何公证发生之前，缺少必要的前置步骤。新增测试只检查字符串顺序，无法证明该发布路径可执行。

## 必须修

### [P1] 在首次 app staple 之前完成该 app 的公证

- 改动位置：`.github/workflows/release.yml:1313-1318`。
- 直接依据：Seal step `:1283-1285` 只有 prepare、sign-app、signature-only 验证。`scripts/release/macos-developer-id.sh:336-347` 的 `sign_app` 只做代码签名；`:367-372` 的 `staple_app` 仅调用 `xcrun stapler staple`，不会提交公证。整个现有脚本唯一 `submit_for_notarization` 调用在 `notarize_dmg`（`:359-364`），工作流直到 `release.yml:1322` 才会运行它。
- 触发与影响：对本次新签名、尚未公证的 app，`:1313` 要获取并装订尚不存在的 ticket。`set -euo pipefail` 会在此终止正式发布，无法到达 DMG 创建及后续唯一公证。旧流程在 DMG 公证后能取得内含 app 的票据；单纯把 staple 前移切断了这一前提。
- 最小修正：复用现有 `submit_for_notarization` / Accepted 轮询逻辑，先将已签名 app 打成可提交的 ZIP 并完成公证，然后 staple 和完整验证同一个 `$APP_PATH`，再创建、签名、公证及验证 DMG，最后验证挂载 app。保持预发布无秘密分支不变，不增加新的发布平台或全局工具。

## 与该阻断直接相关的测试缺口

`tests/releaseWorkflow.test.ts:2492-2497` 新增的是 `indexOf` / `toContain`：它们确实能发现 staple 排在 create 之后或挂载 app 被降成 signature-only，但不验证 staple 前已有 Accepted 的 app。更关键的是 `:2356-2358` 和 `:2448` 还明确禁止 `notarize-app` / `notarize_app`，会把缺失必要前置公证的流程当成正确目标。

修复时同步移除过时的“禁止 app 公证”约束，并补一个不使用真实证书/网络的定向 shell 行为检查：用记录调用的替身执行实际发布片段，未 Accepted 时让 staple 拒绝；断言 app 公证完成 → app staple/完整验证 → 从同一 app 创建 DMG → DMG 公证 → 挂载副本完整验证的真实调用顺序及参数。至少加入“未公证不得继续打包”和“挂载副本 ticket 失败不得通过”的负例。无需全库测试，也不能把替身执行称为正式 Apple 签名证据。

## 已确认正确的部分

- 打包输入是经前序 Seal step 输出的 `$APP_PATH`（工作流 `:1286、1293、1318`），`create-macos-dmg.sh:112-120` 通过 `ditto "$app_path" "$stage/FyAgent.app"` 复制这个输入，再从 stage 创建镜像，没有另选原始 app。
- `release.yml:1326-1334` 明确只读挂载输出 DMG，版本和主可执行文件摘要都从 `$mount_point/FyAgent.app` 读取。完整 verifier 也接收这个挂载路径，不是源 `$APP_PATH`。
- `verify-macos-signed-app.sh:23、33-83` 全程沿传入的 app 路径执行双架构签名身份检查、`codesign --verify --deep --strict` 与 `stapler validate`；未传 `--signature-only` 时票据为必需。挂载 verifier 失败会终止发布。
- source digest 仅覆盖主程序，不等于整包字节证明；但当前入口复制关系及挂载后的完整签名/票据验证已提供本问题所需约束，不建议为此次小修扩建整包审计框架。

## 证据边界

本次结论为固定源码审查；未触发 Apple 服务或签名 runner，未声称发布成功。修复后先做上述最小无秘密行为检查；真实 app/DMG 票据是否有效仍需正式签名 runner 的实际回执。

路径均相对于 `/Users/serendipity/.codex/worktrees/fyagent-fde-reliability/fyagent` 的固定提交 `8118a163`。
