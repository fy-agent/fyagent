# 连续动效实施与验收

## 实际实现与边界

`motion.ts` 统一 `.32,.72,0,1` 空间曲线；原先名不副实的 spring 导出改为
`fySelectionTransition`，SelectionLens、Collapsible 和调用方测试同步迁移。
按压继续复用现有 Motion 手势和受限恢复弹簧，不把业务点击延后。

`dialogPresentation.ts` 是当前 Dialog 内的原生 WAAPI 编排适配，不是新的动画
引擎。固定布局的真实窗口与文字不被拉伸；一个 `contain: layout style` 材质
平面插值位置、尺寸和四角。来源/目标材质与正文分别交接，不读取按钮文案、
输入值或任意属性，不复制 DOM、不截图、不缓存账号内容。

正常展开总长 420ms，前 80ms 保留实际按压反馈，正文在 252–420ms 接管；
返回 360ms，正文最多 80ms 交出，外壳延后 16ms 启动返回，最后 90ms 交还
真实来源。关闭、取消和操作撤销不等待这些装饰阶段。默认即时清理 body/actions；
只有已审核的登录前账号类型选择可以保留 80ms 原始 inert/aria-hidden 呈现，
会话/设备码和 MCP 编辑内容不适用。快速重开仍是新草稿会话。

原生动画开始、结束、部分失败、取消、中途反向和前台失焦均受同一代次检查；
每条已开始的动画都必须清理，Promise 拒绝被处理，Radix 保留唯一模态/焦点
职责。缺少原生动画或减少动态效果时直接收敛，不增加第二套焦点围栏。

## 测试发现与修复

1. 生产 CSS 把 `420ms` 优化成 `.42s`，旧 `parseFloat/1000` 误变成 0.42ms。
   增加严格 ms/s 时间入口，并在实际 production bundle 中断言动画总时长。
2. 可见来源可能因布局边缘半像素舍入而被判为裁剪；最多一个设备像素的容差
   有独立测试，真正越界或滚走仍为中性返回，没有取消可见性判断。
3. Radix 焦点移动与 Router 选中状态提交并非同一步；测试等待真实选中提交，
   未延长超时或修改业务导航。
4. Vite 缓存参数使旧精确 glob 绕过故障注入；network trace 证明原模块实际
   返回 200。改成精确 pathname 匹配，延迟/可选失败/首模块失败原断言均保留。

## 已完成的验证

类型、ESLint 与完整 V2：86 文件、578 项通过。完整四尺寸浏览器：244 项通过，
包含实际按压可见性、正反向关键帧、真实时间、键盘/触控、来源失效、resize、
隐藏路由、敏感表单销毁、快速重开、七页对比度和既有业务流程。
代码指纹记录在 `/tmp/fyagent-round5-continuation-source-snapshot.json`，复核无漂移。

日志：`/tmp/fyagent-round5-final-{types,lint,unit,browser}.log`。先前并发测试导致
Models 场景触及既有 5 秒测试上限；完整串行流程已通过，未扩大超时或屏蔽警告。

生产性能保留每一组，包括失败组，不只选最快数据。第 3 组曾在完整仓库检查
并发期间测得普通模态帧间隔 p95=50ms，原 33.4ms 门槛正确拒绝；同期根测试
开始于 01:29:25，和该模态采样重叠。这不是动画功能失败，也不证明所有低配
设备都满足预算。必须在没有这些并行检查的条件下再核验，不修改阈值。
独立复测日志 `/tmp/fyagent-round5-final-performance-isolated-3.log` 已得到6项通过，
普通/4x模态暖态帧p95均33.4ms，原门槛未改变。另一组
`/tmp/fyagent-round5-performance-acceptance.log` 同样6项通过。
完整 `check:prearchive` 已退出0，日志为
`/tmp/fyagent-round5-choreography-prearchive.log`：根1620通过/1既有跳过，
Rust3472通过/0失败/6既有忽略，格式、类型、Clippy和仓库契约通过。

## SPEC 与未执行边界

已更新 motion-system、visual-language、window-shell、managed-auth 和 quality
中的签名、时序、退出例外、时间单位、测试与失败处理，不保留相互矛盾的旧规则。
参考外部实现的有效时序，不导入外部路由/状态框架；没有记录其本机路径。

未执行真实凭证操作、最低版本原生 WebView/GPU、签名、推送、发布或部署。
浏览器帧间隔、对比度和故障注入只证明受控样本，不是全设备性能或无障碍认证。
