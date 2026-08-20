# 005 安装冲突诊断

- 章节：2.2
- 目标文件：`about-diagnose-conflict.png`、`about-diagnose-conflict-en.png`、`about-diagnose-conflict-ja.png`
- 尺寸：1600×1000
- 主题：同一工具存在多个安装位置
- 语言：zh / en / ja 分别拍摄
- 前置数据：隔离环境中为一个工具准备两条无隐私路径和不同版本
- 界面状态：全量诊断完成，冲突列表展开
- 必显元素：工具名、当前命中版本、两条安装位置、继续或关闭操作
- 隐私要求：只用 `example/profile/tool-a` 与 `example/profile/tool-b`；不显示真实用户名或平台绝对路径
- 验收：读者能看出“多份安装”而不是普通升级失败；状态由真实诊断返回
