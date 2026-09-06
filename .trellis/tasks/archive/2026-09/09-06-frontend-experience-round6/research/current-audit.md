# 第六轮当前事实与缺陷基线

核查日期：2026-09-06。代码基线 `6117a7d9`，父合并 `1dcf8af2`；初始主工作区
干净，无活动任务。研究使用生产构建、1232×700、仓库已有合成IPC夹具；没有
读取或操作用户真实账号，也不将用户截图中的身份内容写入仓库。

## 已复现的缺陷

| ID  | 现象与根因证据                                                                                                                                                                 | 责任位置                                                                                                                   | 验证缺口                                                      |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| F1  | 选中“账号”文字存在且CSS为深色，但被后绘制的玻璃镜片盖住。Tabs.List是display:contents；给这个无盒节点z-index不能抬高实际按钮。浏览器临时给真实按钮z-index:1后文字恢复，颜色未改 | `src/v2/shared/ui/FeatureTabs.tsx:62-77`、`selection-lens.css:9-46`、`app/styles/features.css`                             | 不能仅以DOM存在、elementFromPoint或声明颜色比证明实际文字可见 |
| F2  | 提示词详情在Chromium/WebKit都仅30px；页面CSS把共享SplitPanes的Grid覆盖为Flex，网格列定义失效。临时只恢复display:grid后详情640px，父容器仍922px                                 | `src/v2/pages/prompts/page.css:19-24`、`shared/ui/split/split.css:1-29`、`shared/ui/catalog/CatalogMasterDetail.tsx:22-42` | 无横向溢出不代表有可用宽度；旧测试没有覆盖这个最小可用区域    |
| F3  | 登录第二步按“上一步”，高度574.28125px直接变319.0625px；随后600ms所有样本只有最终高度，没有中间尺寸                                                                             | `src/v2/pages/auth/LoginDialog.tsx`、`shared/ui/Dialog.tsx`的ResizeObserver与settle路径                                    | 入口/退出420/360ms验证不能覆盖同一弹窗内部步骤改变            |
| F4  | 快点并非完全没有按压：Chromium scale最低0.975、回弹峰1.00117；WebKit最低0.975、峰1.00159。存在运动不等于用户能清楚感知                                                         | `src/v2/shared/ui/usePressFeedback.ts:48-96`、`motion.ts`                                                                  | 原测试偏重长按后几何，需快点80–200ms、遮罩接管与真实帧证据    |

两浏览器的F1/F2/F3因果实验一致，页面无pageerror。所有临时CSS仅注入测试页，
没有更改产品代码。诊断脚本与截图位于忽略目录：
`node_modules/.cache/fyagent-round6/inspect.mjs`、`evidence/baseline.json`、
`evidence/{chromium,webkit}-{auth,prompts}-before.png`与对应probe图。
`evidence/tabs-before.png`与`tabs-paint-probe.png`已经逐图复核；后者不是交付截图。
控制台摘要：`/tmp/fyagent-round6-layout-baseline.log`，退出0。

## 有源码证据、尚不能宣称完整性能根因的项

- `SelectionLens.tsx`包含48帧固定几何重测；隐藏页恢复会增加revealKey并从
  零宽高重新展开。`motion.css`又对每次visible的页面播放3px位移和透明度动画。
  这些与用户描述相关，但实际卡顿占比需生产profile及逐帧采样，不能凭代码长度定罪。
- `SplitPanes.tsx`重复实现指针、键盘、body cursor/userSelect和宽度约束。
  当前断点只看window，嵌套容器缩窄和拖动中卸载的清理边界需要补测。
- `tests/v2-browser/support/visual.ts`隐藏文字获取背景，再用声明color重建
  前景。pointer-events:none的遮挡不参与elementFromPoint命中；这类检测可能
  在F1上给出好看的颜色比。它继续作为补充取样，但不能再单独作为遮挡结论。
- 主题参考实现确有原生View Transition圆形clip-path、序号取消和480–640ms
  距离时长。用户报告的中段停顿尚未在本任务归因；不宣称复制实现即可修好。

## 单一前端与离线交付的债务

真实入口为 `src/index.html` → `src/v2/main.tsx`，旧 `src/main.tsx`/`App.tsx`
不在生产入口。该HTML仍带file:重定向，`build:renderer`还生成独立离线预览。
根目录 `deplink.html`被多语言手册作为用户工具宣传；这不只是文件位置问题。
消除范围包括生成脚本、特殊Vite构建模式、重定向、旧测试入口和手册指引。
真正的应用HTML、Tauri深链接接收/校验和自动化安全断言必须保留。

跟踪的TS/TSX/CSS共535文件：当前树177、中立域7、其余旧树351。这是盘点数，
不是351文件都可无条件删除的证明。采用已安装dependency-cruiser读取生产入口；
包含type-only依赖的图有210个src节点，其中跨出当前树的仅7个中立Codex域模块。
相关原始图：`/tmp/fyagent-round6-runtime-graph.json`，另有runtime-only图。
type-inclusive图里UI引用两个feature类型触发runtime规则的记录，不作为运行时
越层缺陷；迁移必须分别看类型与运行时图，不用错误扫描口径制造架构告警。

## 必须保护的合并与测试边界

- `6117a7d9`的Claude目录官方链接与原生隐藏窗口Query retry修复。单一前端
  迁移后的启动测试仍要用真实native-shaped目录DTO，不能只调整宽松的假数据。
- Claude CLI、恢复最近文件修改、认证风险确认与原子回滚属于最近后端合并，
  页面迁移不能移除控件、放宽权限或改变对应命令载荷。
- 根测试中包含原生权限、签名、发布、深链接、配置安全等，不是“旧UI测试”
  的同义词。删旧renderer前必须列出每个测试的保留/迁移/退役原因。
- 浏览器WebKit通过不等于最低macOS版本WKWebView验证；Chromium也不等于
  Windows原生权限、WebView2安装或真实GPU合成验证。
