# 动效方案

## 复用与职责

Motion 12.23.25负责spring、tap和可打断动画；Radix负责modal、focus、语义和卸载协同。共享owner输出按角色的transition/variants，不允许每页直接复制物理参数。普通CSS交互采用同源token；不生成第二套弹簧求解器或全局鼠标监听器。

`primitives.tsx`把Dialog/ConfirmDialog与按钮分成具名模块是明确职责需求，不为了改名而搬动所有Page。在对应API稳定后将全部消费者更新到新owner；检查测试/static guards/SPEC和上下文的路径，不留下遮蔽反向依赖的barrel。

## 来源

引入显式可选trigger/originRef契约或共享trigger wrapper，所有用户发起的打开路径在事件当时记录element/rect。动画只消费几何，不复制按钮内容/label或业务状态。鼠标和键盘共用真正触发元素，不用最后一个全局click推断来源。没有有效来源的启动错误/自动状态弹窗以中心轻淡入处理。

优先合成transform与opacity，保持布局尺寸及文字清晰。不要大幅压扁整张表单来模拟按钮变形；如需完整位移/尺寸映射，可由独立材质层呈现，内容只做有限过渡。退出回到原源；源被删除、滚走或路由隐藏时中止空间返回而非跳到错误位置。

## 节奏调优起点（待实测）

| 角色                   | 起点                                           | 限制                                               |
| ---------------------- | ---------------------------------------------- | -------------------------------------------------- |
| 按压                   | 立即反馈，80-100ms轻收缩；220-280ms spring恢复 | shrink约0.97-0.98，最大scale≤1.005；布局box不动    |
| selection/navigation   | 180-260ms低位移平滑过渡                        | 不等待退出才加载下一页；不能造成目标双交互         |
| dialog                 | 进入380-440ms，退出280-340ms                   | 入场明显减速、轻/无overshoot；来源失效使用中性淡变 |
| popover/toast/collapse | 180-300ms依内容/距离                           | 保留布局和可访问性；背景blur不逐帧大幅重算         |

数值是与用户“柔和而不迟钝”目标对应的起点，不是官方认证的唯一舒服参数。采用Carbon productive/expressive分工，物理spring和duration/bounce不混杂覆盖；最低WebView不支持的新API不可擅自加入。

## 生命周期

比较Radix官方CSS presence与Motion asChild/presence组合，选择能满足来源移动、可中断、隐藏清理与第三轮焦点约束的最小实现。退出完成仅处理展示，不作为native作业完成信号。rapid reopen取消旧退出/restore callback，modal移交focus由Radix管理，不额外focus trap。

减少动态效果来自已有useReducedMotion和CSS media，必须实时响应；强制颜色/减少透明度由surface任务规则处理，动效不绕过它。
