# 内容尺寸与交互连续性

保留Dialog已验证的420/360ms来源运动、即时关闭/秘密清理与Radix焦点owner。
内容步骤变更不同于窗口resize：复用Motion或WAAPI只动画共享尺寸边界和内容
透明度，不scale表单，不把本次步骤当作重新打开。显式步骤identity使新内容
交接可测；中途下一步/上一步或关闭取消从当前尺寸收敛，草稿保持同一会话。

快点按压先可见收缩再有限弹性恢复；Motion唯一transform写入者，业务点击立即。
重访已存在的镜片不从零宽高重播；装饰层不覆盖文字、不持续滤镜动画。去除
确实不必要的固定48帧重测，保留observer对实际布局变更的校准与可中断移动。
性能分别测路由就绪和装饰完整帧，不拿加载时间替代运动稳定。

不延后权限/取消/启动就绪，不制造动画队列、全局最近点击或新插值引擎。

## 实施前边界复核

当前业务步骤改变根内容高度后直接调用打开动画的settle，574px→319px没有
中间帧。来源开合继续只动画独立材质；同会话内容调整则使用浏览器WAAPI动画
固定定位Dialog的width/height，真实文字只重排而不缩放。正文轻量淡变，底部
按钮保持在同一flex窗口内，因此按压反馈不会被整层透明度隐藏。显式
presentationKey用于同尺寸步骤和中途反向；ResizeObserver捕获其他真实尺寸
改变，但自身正在进行的尺寸动画只更新观测值，不能重新启动它。

相邻页面与主布局不参与重排。真实viewport变化、减少动态效果或隐藏直接收敛。
关闭时保留当前几何给现有来源返回轨道，立即销毁交互/敏感内容；不保留旧表单。
这个选择比为每一个表单子元素加FLIP反向scale更小、更可验证，但仍有布局成本，
必须测生产暖态前后步骤，不能用路由加载时长代替。

镜片初次展示/保活回访直接取得真实选中位置，不从零尺寸重新生长；用户选择才
从当前几何运动。布局观察驱动的移动即时校正，不对每帧的新目标重新启动tween。
用ResizeObserver观察所属布局块；缺少该API时只观察真实折叠节点的style变化，
不使用固定48帧轮询。语义选中与焦点不改变。

按压复用Motion现有press/styleEffect/spring，把目标压缩调到0.96并让恢复曲线
形成受限的可见回弹。未接入的业务按钮复用无样式PressableButton；已被镜片测量
的列表仅动画现有文本子层，绝不让按压污染几何来源。

一手依据：

- https://developer.mozilla.org/en-US/docs/Web/API/ResizeObserver — 回调在paint前；须防止自触发循环。
- https://developer.mozilla.org/en-US/docs/Web/API/Animation/cancel — cancel撤销效果并拒绝finished，所有句柄要处理拒绝。
- https://motion.dev/docs/performance — 隔离几何仍须测低性能条件，不宣称所有布局动画都是GPU合成。
- https://motion.dev/docs/react-layout-animations — layout用transform/scale，需要防止表单文字失真。

## 上游样式订阅生命周期缺陷

对真实WorkBuddy页面按压抓取仅限按钮自身的DOM属性，确认手势已接受、非禁用、
可见且非减少动态效果，但视觉样式仍停在none。已安装motion-dom12.23.23的
createEffect把subscriptions数组置于所有实例共享的闭包中：任意一个清理函数
都会清理此前所有按钮的样式订阅。独立双控件回归先确认第二个能变为0.98，
清理第一个后第二个无法变为0.96，直接复现此缺陷，而不是曲线或选择器问题。

上游main和官方npm发布物12.35.2、12.43.0已将数组移入每次调用内，12.24.0
发布物仍有旧实现。采用维护者同一主版本最终稳定分支framer-motion12.43.0，
不升级13、不复制上游实现、不添加本地node_modules补丁。重新运行全量
Presence/Radix、镜片、按压隔离和实际生产预算，锁与依赖审计一起检查。

- https://raw.githubusercontent.com/motiondivision/motion/main/packages/motion-dom/src/effects/utils/create-effect.ts
- https://raw.githubusercontent.com/motiondivision/motion/main/CHANGELOG.md
- https://registry.npmjs.org/framer-motion/12.43.0
- https://registry.npmjs.org/motion-dom/12.43.0

共享折叠进一步直接使用Motion声明式`animate={{height: open ? "auto" : 0}}`，
移除手动scrollHeight采样、历史高度缓存、Promise回写auto和代次计数。这避免
WebKit中手动末帧height回写争用，并让开放内容按自然高度响应后续数据。Radix
保留同一内容节点和展开语义，关闭立刻inert/aria-hidden，无旧表单快照。
依据：https://motion.dev/docs/react-animation#value-type-conversion 。
