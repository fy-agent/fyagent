# 最终结果（2026-09-30，UTC+8）

本任务以 FyAgent **v0.4.9** 正式发布收尾。v0.4.8 tag 存在但没有对应 Release。

## 合并记录

| PR | 内容 | 必需 CI | 合并提交 |
| --- | --- | --- | --- |
| [#199](https://github.com/fy-agent/fyagent/pull/199) | 下载源、版本检测与安装修复，版本 0.4.8 | PR run 36611927729 通过（首次有一个与本改动无关的 WebKit 用例超时，只重跑失败 job 后全绿）；merge_group run 36620594940 通过 | `8566e62cd221a2b628fa9d7cef5abd0ba8125e43`（04:04，merge queue） |
| [#200](https://github.com/fy-agent/fyagent/pull/200) | 版本改为 0.4.9，补齐中英文 release notes | PR run 36636366079 通过；merge_group 通过 | `9519d288a27f07ba092bca5d49c588b1cd8aebce`（06:47，merge queue） |

两次合并都经 merge queue 完成，没有用 admin 绕过。新增的 14 个 Rust 单测在 #199 CI 的 Windows 和 macOS backend job 中全部通过。

## 发布记录

- **v0.4.8：打了 tag，但没有 Release。**
  - 附注 tag 指向 `8566e62c`。Preflight run 36624351650 通过。
  - formal run 36629904293 在 “Fail closed before the one-time publish” 这一步停止，报错 `English v0.4.8 Release Notes are missing`，原因是 #199 没有带 `docs/release-notes/v0.4.8-en.md`。
  - 没有生成 Release，也没有草稿。tag 按 release contract 保持原样，没有移动，也没有重建。
- **v0.4.9：已发布。**
  - 附注 tag 指向 `9519d288`。Preflight run 36641757086 通过，formal run 36647253696 通过。
  - Release 地址：https://github.com/fy-agent/fyagent/releases/tag/v0.4.9 。08:39 发布，不是草稿，不是预发布，已标为 Latest。
  - 共 7 个附件：3 个安装包，加上 `download-manifest.json`、`build-metadata.json`、`signing-status.json`、`artifact-attestation.sigstore.json`。
  - 3 个安装包重新下载后核对了 SHA-256。结果与 download-manifest、signing-status 以及 provenance（SLSA v1）的 subject 摘要一致：

| 安装包 | 大小 | SHA-256 |
| --- | --- | --- |
| `FyAgent-0.4.9-macOS.dmg` | 29,286,288 | `ad9b542004bae01f90c59b1a6486f37aec82fffb8c6dd74effbe7c2bf15d7994` |
| `FyAgent-0.4.9-Windows-x64-setup.exe` | 9,701,690 | `0c9134373ed8dcae197bb007dd7875dd5b05edcbfb066ff8b8fafc90eb6a7356` |
| `FyAgent-0.4.9-Windows-arm64-setup.exe` | 8,608,000 | `b75dd3280a7223260fd6f889bfb0d0ce5ca44ca394a4ecfc427738eb177333c6` |

  - Windows 安装包的 signing-status 为 `mode=unsigned`，与 v0.4.5 相同。

## 未完成，转为后续跟进

以下各项都没有在真机上执行。它们不算已验收，归档后作为后续跟进保留：

1. Windows x64、Windows ARM64 和 macOS 真机验收：各产品的全新安装、已有安装升级、多 owner、取消或失败后的恢复、重扫、启动（对应 implement.md D 组两项未勾选的内容）。
2. OpenCode Desktop 2.x 在 Windows 上的安装目录 `@opencodedesktop\OpenCode.exe`。目前只根据官方安装包离线推断，还没在 Windows 真机上确认。
3. 0.4.9 的三个安装包在真机上的安装和运行。
4. 本机没有 Apple SDK，无法对 macOS 目标做本地交叉检查。macOS 只有 CI runner 的结果。
5. 已有问题，不是本任务引入：主 crate `fyagent` 和 `fyagent-user-helper` 在 Linux 开发主机上编译不过，和 development-environment 契约的说法不一致，需要另开任务。
6. 已有问题，不是本任务引入：通用 CLI 就绪卡片还没有透传 `latest_authority`（external-agent-sources 规范中已标为后续）。

## 归档过程

- 归档前，在直接会话 `session:junshi-fy-maint-0929` 下执行了 `mise run check:contracts:prearchive --exclude-active-task .trellis/tasks/09-30-agent-source-maintenance`。
  - 结果：supported-platform、version（0.4.9）、task/docs/lockfile 这几项契约通过。
  - contract-tests 为 660 passed / 3 skipped / 2 failed。这 2 个失败是 Linux 开发主机的环境问题，和本改动无关：
    - `miseTaskContract` 报 `Unsupported test host: linux`；
    - `repositoryGovernanceScan` 在临时 fixture 仓库上的 history 用例失败。
  - native-fetch 4 个测试通过。
- 随后执行 `task.py archive --no-commit`，status 写为 completed，任务移到 `.trellis/tasks/archive/2026-09/`，当前会话指针已清除。
- context 清单和 meta 里的路径都已改为归档后的位置。
- 归档后，在不带排除参数的情况下执行 canonical `mise run check:contracts`：supported-platform 通过（3109 个当前文件），其余结果与归档前相同，也只有上面 2 个环境失败。
- 这两个用例都要以受保护主线的必需 CI（`CI / Required`）结果为准。
