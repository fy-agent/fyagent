# 滚动核查

基线长列表在Chromium/WebKit均失败：Skills及MCP的最后一个项目无法由wheel
到达。六项首轮中四失败，发现页两项通过；未把已正常的发现页声称为复现缺陷。
共同根因为中间FeatureTabPanel未传递高度，父规则又将它设为不收缩的auto，
分栏跟随内容膨胀并被上层裁剪。显式必填layout角色修复共享边界，移除Skills
发现页绕行覆盖。相同六项回归全部通过，随后扩展双主题及页签回访。

页面owner清单：Agents/Models沿用CatalogMasterDetail/内容视口；Auth的账号与
连接页签使用workspace+既有分栏；Skills已安装/发现和MCP已安装/发现采用明确
workspace；Prompts沿用SplitPanes；Memory长记忆/日记页签为workspace。普通
Agent配置和MCP编辑器内部表单页签为flow。没有新滚动库、wheel监听或业务改动。

原Skills CSS断言要求旧发现页补丁，按新共享owner更新。Git记录为page.css，
保留该准确大小写，不按macOS大小写不敏感的读取结果推断文件名。

组合回归还暴露两项相邻问题：旧镜片测试在Router完成激活之前记录隐藏页面
零宽度，现从第一可见帧开始而非等动画结束；快点按压若在0.985以下释放会
立即回弹，偶发最低仅0.976，现完成同一受限压缩目标后恢复（业务点击仍立即）。
这些属共享交互回归，不放宽原有0.974/1.005阈值。原失败记录保留。
Memory复制草稿测试首次全量有一次焦点被旧close帧取回，定向18项及第二次
全量通过；将该明确异步焦点竞争交下一来源子任务做确定性回归，不能靠重试宣称消除。

补充的其余五路由原生滚动探针尚在验收。当前失败来自夹具对未知命令先调用
旧fixture、以及只查页面内部忽略合法外层viewport；已逐项修正并增加Models
80条ID。不得在该新增覆盖完成前将滚动任务归档。发现同目录另一个验证进程
正在跑性能；不终止它，也不并发运行新的压力测试，以免混淆采样。

## 接续核验

重新读取并审查全部既有工作区改动后，严格类型/lint及完整生产启动+浏览器
门禁通过：376项浏览器回归，覆盖四桌面尺寸和WebKit关键路径。
日志为`/tmp/fyagent-round7-resume-browser.log`。完整预归档门禁
`/tmp/fyagent-round7-scroll-final-gate.log`退出0；单元1550通过、1既有跳过，
Rust3495通过、0失败、6既有忽略。日志中的单次Memory测试act环境警告
作为后续弹窗生命周期排查输入保留，不全局屏蔽console或关闭动画。

生产性能串行门禁10项通过，日志`/tmp/fyagent-round7-scroll-performance.log`。
导航普通/4倍CPU回访p95为28.1/46.7ms；普通暖态开合帧p95为33.4ms。
该性能数据不包含操作系统输入延迟，不代替原生WebView/GPU验证。
