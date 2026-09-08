# 共享绘制与分栏

F1不是简单文字颜色错误：display:contents列表没有可靠层叠盒。选中装饰层
只绘制底材，实际语义按钮/文案位于其上；不在账号页打特例，不以pointer命中
代替真实像素检查。F2是局部display:flex覆盖共享Grid，先消除此覆盖并回归。

SplitPanes保留当前接口和内容状态，由react-resizable-panels负责指针/键盘/清理。
自身只保留产品最小尺寸、窄容器布局和标签；避免在断点重建表单。宽度依据实际
容器，不按window猜测；三栏保留宽屏三列与窄屏堆叠可达语义。若库的已核实限制
会导致草稿重建，不强接入，记录阻碍再以最小owner修复完成该边界。

重点写集：shared/ui/{FeatureTabs,SelectionLens,split,catalog}及相关CSS，
pages/prompts/page.css、对应组件/浏览器测试、spec的容器/视觉规范。
不改变提示词业务CRUD、账号权限、目录顺序或后端命令。
