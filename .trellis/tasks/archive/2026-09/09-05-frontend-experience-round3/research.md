# 官方依据与初始调查

检索日期：2026-09-05。

- Radix Dialog：https://www.radix-ui.com/primitives/docs/components/dialog 。复用焦点约束、Title/Description 和受控关闭，不重造焦点陷阱。
- Radix Alert Dialog：https://www.radix-ui.com/primitives/docs/components/alert-dialog 。破坏性确认应有明确取消路径和可辨识动作；先核对现有 Dialog 的兼容性，不强换整套 primitive。
- Material Typography：https://m3.material.io/styles/typography 。采用角色化层级而非复制完整移动端字号体系；本产品为密集桌面工具，保留可读正文、控制标题尺寸。
- Tauri 窗口：https://v2.tauri.app/learn/window-customization/ 、https://v2.tauri.app/reference/config/ 。已有 `visible:false`；需要修正实际 show 时序，不新增启动屏。

现状必须通过运行页面、计算样式和源码同时核验。官方原则不替代本产品可用性测试；初次发现不是已完成修复。

补充核对：React Suspense https://react.dev/reference/react/Suspense 、useLayoutEffect https://react.dev/reference/react/useLayoutEffect 、Tauri隐藏窗口与初始化示例 https://v2.tauri.app/learn/splashscreen/ 。就绪来自内容提交而非外壳mount；不增加启动屏，不把固定延迟当完成信号。Radix声明及本机安装源码共同验证了autoFocus与开启事件的次序。
