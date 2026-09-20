# 当前流程演示

运行 `mise run demo:capture`，使用已有 Chromium 与浏览器 fixture 采集当前页面。
默认输出是调试产物。界面整合稳定后，可运行：

```sh
FYAGENT_DEMO_CAPTURE_KIND=candidate mise run demo:capture
```

每次采集在 `artifacts/current-demo/<时间戳-进程号>/` 新建独立目录，避免旧截图、
字幕和新视频混用。任务只使用 4198 端口，不复用其他开发服务器。缺少浏览器时，
先按项目现有浏览器测试环境准备；脚本不会自动安装浏览器或接入个人 profile。

本任务不连接真实账号、不运行安装器，也不写用户的 Agent 配置。页面使用真实
Renderer 组件和操作流程；保存成功、失败与恢复结果来自明确配置的 fixture。
这些结果不证明原生文件保存、实际回滚、账号授权、Windows 实机或生产可用性。

| 文件                                             | 展示内容                         | 证据边界                             |
| ------------------------------------------------ | -------------------------------- | ------------------------------------ |
| `01-purpose.png`                                 | 首次用途选择                     | 选择用途或跳过引导                   |
| `02-recommendations.png`                         | 办公用途推荐                     | 实际推荐页面                         |
| `02-existing-config.png`                         | 已有配置的保留或替换选择         | 只读观察后的选择，尚未创建写入计划   |
| `02-software-setup.png`                          | 选中软件的安装和配置入口         | 单软件卡片，进入模型配置前无写入     |
| `03-save-preview.png`                            | WorkBuddy 单次配置预览           | 预览创建后仍未发送 apply             |
| `04-recovered-failure.png`                       | 失败后显示已恢复                 | fixture 返回的恢复状态               |
| `05-saved-result.png`                            | 成功保存且凭据已清空             | 另一组独立 fixture 返回的成功状态    |
| `current-flows.webm` / `current-flows.zh-CN.vtt` | 首次使用、推荐、预览、失败结果   | 当前页面操作录屏与对应字幕           |
| `saved-result.webm` / `saved-result.zh-CN.vtt`   | 填写、确认、成功结果             | 独立成功场景，不是失败场景的真实重试 |
| `index.html`                                     | 两段视频、字幕及七张图片的播放器 | 需通过本地 HTTP 打开                 |
| `manifest.json`                                  | 完成清单、来源和校验值           | 只有采集及播放器检查完成后写入       |

原始 WebM 保存在该次目录的 `raw/`。`test-results/` 是 Playwright 的测试输出，
其中失败或未完成的运行不应被当作完整演示包。旧脚本可能留下直接位于
`artifacts/current-demo/` 下的文件；它们不是新运行的输出。

任务会检查密码输入确实遮盖、普通输入与页面文字不出现演示密钥或用户绝对路径、
预览和 apply 各只有一次、apply 仅提交预览的 opaque identity，并阻断浏览器外部
HTTP 请求。演示不调用真实安装、账户或配置写入入口。

采集后会读取两段编码媒体的真实时长，以录制结束时间校准字幕，并在每个稳定
画面两侧留 250 ms。播放器逐条验证字幕 cue、从头播放两段视频至结束，确认图片
能加载。该自动回读证明媒体与字幕可播放，不替代人工逐张检查和整段观看，也不是
像素一致性或无障碍审核。

使用任意已有的静态 HTTP 服务，以该运行目录为根打开 `index.html`。采集任务的
自动回放使用 Vite 的 `/@fs` 文件访问路径；普通 `/artifacts/...` 地址会被项目的
`src` 根目录回退为应用页面，不能用作播放器地址。不要仅以 `file://` 打开，因为
浏览器可能拦截独立字幕文件。交付时保留同一目录中的视频、VTT、截图、播放器和清单。

清单记录版本、源码提交、相关 Renderer / fixture / config 与原生目录文件的 SHA-256、
采集前后源码摘要、素材哈希、截图所属视频与 cue、录制时钟偏差和播放检查结果。
相关未跟踪源码也计入摘要。`sourceStable: false` 表示采集期间源码发生变化；调试
运行仍保留这个事实，候选运行则失败且不写完成清单。`trackedSourceDirty: true` 或
`untrackedSourceFiles` 非空时，即使 `sourceStable: true`，也只是本地候选证据。
正式采用前应在完成整合的干净源码提交上重新采集，核对清单并人工观看。
