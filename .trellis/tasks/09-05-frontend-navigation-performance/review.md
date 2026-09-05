# 导航性能与生产运行复核

## 根因与边界

首次生产测试不是慢，而是不能初始化：vendor-react 静态导入 vendor-shared，
后者在 React 尚未初始化时读取 useLayoutEffect。Vite开发服务器和旧的单文件预览
都没有这条分包执行路径。已保留失败日志，并用生产启动/七路线测试防回退。
复用Rollup命名入口及其依赖闭包，移除手写node_modules分类算法，不增加框架或预算。

另一个可复现问题是父route每次创建新的CommittedPrimaryPage，导致已隐藏且没有
路由订阅的页仍渲染。新隔离单测在旧代码失败；路由ID边界memo后通过。不屏蔽真实
context/Query/本地更新。SelectionLens登记context稳定化，位置由transform合成，
不在每一帧改left/top触发布局；尺寸和第三轮隐藏/焦点契约保持。

## 性能证据

因原构建无法运行，数字基线明确是“仅修复分包、尚未优化渲染”，不能冒充旧生产
性能。相同生产构建、1232×700、串行、七页各六次回访共42样本，不包括OS输入分发，
计时终点是目标DOM可见后的下一帧，不是原生合成器首帧或动画结束。

| CPU成本 | 基线p50/p95 | 改后p50/p95 | 平均JS时间 基线→改后 | 平均layout 基线→改后 |
| ------- | ----------- | ----------- | -------------------- | -------------------- |
| 1×      | 36.3/39.0ms | 36.6/39.9ms | 4.667→3.973ms        | 0.721→0.693ms        |
| 4×      | 47.6/53.4ms | 46.3/53.0ms | 22.481→18.493ms      | 3.274→2.907ms        |

1×无longtask，4×各有一个74ms样本（包含首次访问）。帧p95变化在刷新周期和噪声
内，不能宣称显著切页倍数加速；减少了不必要渲染与JS工作，普通样本满足100ms目标。
没有在该小型确定性数据集复现用户全部原生卡顿，不据此重写所有大Page或强制eager挂载。

日志：`/tmp/fyagent-round4-perf-{baseline-profile,after}.log`，测试输出含CPU profile。
后续材质/动效整合后还要重测，避免滤镜抵消收益。

## 参考与检查

- https://rollupjs.org/configuration-options/#output-manualchunks：依赖归组和执行顺序风险。
- https://react.dev/reference/react/memo：只挡不变props，context仍然正常更新。
- https://motion.dev/docs/performance：合成transform优先于逐帧布局属性。
- 新增隔离测试与既有镜片测试11项通过；生产启动和两组性能测试3项通过。
- 完整V2 76文件/534项通过；生产smoke与184项开发浏览器回归通过；完整
  `check:prearchive --exclude-active-task .trellis/tasks/09-05-frontend-navigation-performance`
  退出0。没有新增豁免、改预算、跳过失败用例；新配置进入CI变更分类。
