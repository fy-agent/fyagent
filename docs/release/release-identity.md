# Windows 发布身份与恢复（草稿）

本文是 #67 的公开身份表。它不含密钥、PFX、PIN、OTP、token 或 SimplySign 密码。
身份行仍是占位时，仓库变量 `FYAGENT_WINDOWS_SIGNING_MODE` 必须保持 `unsigned`。
不要因为本机已经能连上 SimplySign，就把默认模式改成 `provider`。

Windows 正式签名路径是 **Certum／SimplySign**，沿用现有 adapter 边界：

`prove-windows-preflight`（unsigned）与 `sign-windows-formal` → `seal-windows-formal` 分开。
不另开 Azure Trusted Signing 或第二条流水线。adapter 文件是
`scripts/release/windows-signer-certum-simplysign.ps1`。

本机 AIMaster 上的 SimplySign／Certum 操作只作为参考。它不是 CI 正式证据。
正式证据只来自 `fy-agent/fyagent` 的 Release 工作流：x64 与 arm64 各自签名、各自在干净 runner 上复验。
验收命令是 `signtool verify /pa /all /v /tw`，只接受退出码 0。缺 publisher、证书 SHA-256、RFC 3161 时间戳、证书链或安装包 SHA-256 时，现有 seal 逻辑 fail-closed。

## 身份表

两行必须使用同一张 Certum 代码签名证书。聚合步骤会拒绝 x64 与 arm64 的 publisher 或证书不一致。
发布者／证书到期已填；恢复人仍缺第二人，因此仍不能打开 provider 模式。

| platform | publisher | accountOwner | maintainer | approver | recoveryContact | expiresAt | rotateBefore | revocationStatus |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| windows-x64 | Hangzhou Fuyu Talent Service Co., Ltd. | William Wang（Certum／SimplySign 账户 wq24100@qq.com；PIN／OTP 不入库） | 项目经理席（仓库 adapter／身份表／流水线） | William Wang | 待补第二恢复人（禁止与 approver 单点相同后仍未双人） | 2027-11-02T23:59:59Z | 2027-08-02T00:00:00Z | good |
| windows-arm64 | Hangzhou Fuyu Talent Service Co., Ltd. | William Wang（Certum／SimplySign 账户 wq24100@qq.com；PIN／OTP 不入库） | 项目经理席（仓库 adapter／身份表／流水线） | William Wang | 待补第二恢复人（禁止与 approver 单点相同后仍未双人） | 2027-11-02T23:59:59Z | 2027-08-02T00:00:00Z | good |

公开证书指纹（仅选择／核对用，不是私钥）：

- Thumbprint SHA-1（`certificateSha1`／signtool `/sha1`）：`4166b014a2bac1fd2a286127dcd4b4e34695a598`
- Issuer：Certum Code Signing 2021 CA
- EKU：代码签名
- 本机 AIMaster 已 Connect 验证过；**仍不等于** CI／干净机正式证据。
- `recoveryContact` 仍标「待补第二恢复人」→ **checklist 第 3 条未齐，禁止把 `FYAGENT_WINDOWS_SIGNING_MODE` 改成 `provider`**。

字段含义：

| 字段 | 含义 |
| --- | --- |
| platform | 逻辑安装包架构。NSIS 启动器的 PE Machine 可以仍是 x86；签名按 x64、arm64 分作业执行。 |
| publisher | 证书 Simple Name，必须与仓库变量 `FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER` 一致。 |
| accountOwner | Certum／SimplySign 账户的所有者。负责账户存续，不把私钥交给 CI。 |
| maintainer | 维护本仓库签名 adapter、身份表和发布流水线的人。 |
| approver | 批准把签名模式从 `unsigned` 改为 `provider`、以及批准轮换的人。不能与唯一恢复人是同一单点。 |
| recoveryContact | 账户或证书不可用时的恢复联系人。恢复走 Certum 账户流程，不靠仓库里的 PFX。 |
| expiresAt | 证书 `notAfter`，ISO-8601 UTC。 |
| rotateBefore | 必须开始轮换的时间，早于 `expiresAt`。 |
| revocationStatus | `unknown`（尚未签发或尚未登记）、`good`、`revoked`。只有 `good` 才能进入正式签名。 |

## 凭据放在哪里

私钥不进仓库，也不进下面这张表。`certificateSha1` 只是选择证书的公开指纹，仍然只通过 Actions secret 注入，不写进 Git。

| 名称 | environment | allowedWorkflow | allowedRefPattern | 轮换时做什么 |
| --- | --- | --- | --- | --- |
| `FYAGENT_WINDOWS_SIGNER_CREDENTIAL` | GitHub Actions repository secret。只进入 `sign-windows-formal` 作业进程，再由 `windows-signing.mjs` 传给 adapter 子进程。seal 作业看不到它。 | `.github/workflows/release.yml` 的 `sign-windows-formal` | 正式发布：`refs/tags/vX.Y.Z` 的 tag push，或对该 tag 的 `workflow_dispatch` 且 `mode=formal`。预检 `refs/heads/main` 不接收此 secret。 | 见下方轮换步骤。用新值覆盖 secret，不把旧值抄进工单或 Git。 |
| `FYAGENT_WINDOWS_SIGNER_ADAPTER_BASE64` | GitHub Actions repository secret。内容是 adapter 脚本的原始字节，不是私钥。作业把它写到临时绝对路径 `.ps1`，赋给 `FYAGENT_WINDOWS_SIGNER_ADAPTER`，结束后删除。 | 同上，仅 `sign-windows-formal` | 同上，仅正式 tag ref | adapter 脚本变更后，用仓库里该文件的精确字节重新生成 secret。不要改成 `vars.FYAGENT_WINDOWS_SIGNER_ADAPTER`。 |

凭据 JSON 只有这些字段，且必须是单行、无首尾空白：

```json
{"provider":"certum-simplysign","timestampUrl":"http://time.certum.pl","certificateSha1":"<40-lowercase-hex-thumbprint>","signtoolPath":"<optional absolute path to signtool.exe>"}
```

`signtoolPath` 可以省略。时间戳 URL 只能是 `http://time.certum.pl`（RFC 3161）。`http://timestamp.certum.pl` 是旧的 Authenticode 时间戳协议，adapter 会拒绝。未知字段（`password`、`pin`、`otp`、`token`、`pfx`）一律拒绝。

两个指纹不要混用：

- `certificateSha1`：给 `signtool /sha1` 选择 SimplySign 证书，40 位小写十六进制。
- `FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256`：证书原始字节的 SHA-256，64 位小写十六进制。seal 用它核对签名者，不把它当作 `/sha1` 参数。

## 打开 provider 之前的检查清单

全部满足之前，`FYAGENT_WINDOWS_SIGNING_MODE` 保持 `unsigned`：

1. 身份表不再有「未填写」，两行证书策略一致，`revocationStatus` 为 `good`。
2. `expiresAt` 晚于当前时间，`rotateBefore` 早于 `expiresAt`。
3. `accountOwner`、`maintainer`、`approver`、`recoveryContact` 已指定，恢复不依赖一个人。
4. `publisher` 与 `FYAGENT_WINDOWS_SIGN_EXPECTED_PUBLISHER` 相同。
5. 证书 SHA-256 已写入 `FYAGENT_WINDOWS_SIGN_EXPECTED_CERTIFICATE_SHA256`。
6. `FYAGENT_WINDOWS_SIGNER_ADAPTER_BASE64` 是 `scripts/release/windows-signer-certum-simplysign.ps1` 的精确字节（LF，与 Git 中一致）。
7. `FYAGENT_WINDOWS_SIGNER_CREDENTIAL` 已按上面的 JSON 写入 secret。SimplySign 虚拟智能卡已在签名 runner 上可用；脚本不弹出 PIN、也不保存 PIN。
8. 干净 Windows x64 与 arm64 机器上，对两个安装包执行 `signtool verify /pa /all /v /tw`，退出码都是 0。本机 AIMaster 的结果不能代替这一步。
9. 然后才把仓库变量 `FYAGENT_WINDOWS_SIGNING_MODE` 设为 `provider`。

## 离线签名交接

AIMaster 不需要注册成 runner，签名凭据也不进入 GitHub。正式 Release 运行上传的
`raw-windows-x64` 和 `raw-windows-arm64` 是唯一原始包。本机用
`scripts/release/windows-signer-certum-simplysign.ps1` 签名后，把
`signing-return.json` 和两个已签名安装包做成一个不含其他文件的提交，再从
`main` 触发 `.github/workflows/windows-offline-sign-return.yml`。

该工作流重新下载那一次 Release run 的原始包，核对 run id、attempt、commit、
架构和原始包 SHA-256，然后调用现有 `verify-sealed`。公开策略在
`docs/release/windows-offline-signing-policy.json`：publisher 为
`Hangzhou Fuyu Talent Service Co., Ltd.`，证书 RawData SHA-256 为
`df97c8316288564fa123a9be0c239dd3510ff23c59699b7807aa8ac7861ddb53`。
未签名候选失败。这个工作流不创建 GitHub Release，也不读取 Release 附件。
它不代替上面的清单，也不把 `FYAGENT_WINDOWS_SIGNING_MODE` 改成 `provider`。

## 正式发布收回已密封的离线包

当发布提交包含 `docs/release/windows-formal-offline-admission.json`，且其中 `version` 等于该提交的稳定版本时，正式 tag 运行必须收回清单点名的那一对已密封 Windows 安装包。预检仍重新构建未签名包，不发布。清单缺失时，正式发布仍走原来的 `rebuild` 路径。

收回时全部成立才继续，任何一条失败都停止，不会改发未签名 Windows，也不会 `make_latest`：

- 正式 tag 指向含有这条门禁的提交。GitHub 从 tag 提交读取 workflow，不能在只含旧 workflow 的 artifact 源提交上打 tag 来执行新门禁。
- artifact 源提交是正式提交的祖先。两边去掉 workflow 允许清单之后的 git 树哈希一致，差异路径都在允许清单内。`git ls-tree` 和 `git diff` 必须使用 `core.quotePath=false`；带引号的路径不能当成另一棵树。
- `build-windows`、`prove-windows-preflight`、`sign-windows-formal`、`seal-windows-formal`、`build-macos` 除准入闸门外保持原样，macOS 签名和公证步骤不改。
- 重新下载清单中的 preflight attempt 与 offline-sign-return attempt。最新 attempt 必须仍是清单里的 attempt。原始包、已签包、publisher、证书 SHA-256 与 `windows-offline-signing-policy.json` 一致，并再次执行 `verify-sealed`。未签名、缺文件、错 run、错 attempt、错 SHA、错哈希都拒绝。
- 这条路径不读取 signer secret，不把 `FYAGENT_WINDOWS_SIGNING_MODE` 改成 `provider`。

v0.4.15 的清单绑定 preflight `42421033643` attempt 1 与 offline-sign-return `42424610543` attempt 1，artifact 源为 `4f5e0543bae400e0c4447c10c164cd7e285f01df`。正式 tag 打在合入本门禁之后的提交上。原始包在 2026-11-15 06:50（Asia/Shanghai）过期，sealed 包在同日 07:18 过期；过期后这次收回失败，不能改走未签名发布。没有 GitHub Release 时，现有合同仍允许把 tag 指到新的合格提交后重跑。已发布的 Release 不删除、不改附件。

## 轮换步骤

1. approver 在 `rotateBefore` 之前安排轮换。
2. 向 Certum 申请新的代码签名证书。新的 publisher 与证书 SHA-256 先改本表，再改仓库变量。
3. 在受控环境里生成新的凭据 JSON。不提交，不贴进聊天记录。
4. 用新值替换 `FYAGENT_WINDOWS_SIGNER_CREDENTIAL`。
5. 若 adapter 有改动，同时替换 `FYAGENT_WINDOWS_SIGNER_ADAPTER_BASE64`。
6. 下一次正式 tag 的 x64、arm64 seal 证据必须显示同一张新证书，且带时间戳 EKU。
7. 旧证书按 Certum 流程吊销或停用后，把旧行的 `revocationStatus` 改为 `revoked`。
8. 丢失 SimplySign 设备时停止签名，保持 fail-closed，通过 `recoveryContact` 走 Certum 账户恢复。不为了恢复而在 CI 存放可导出的 PFX。
