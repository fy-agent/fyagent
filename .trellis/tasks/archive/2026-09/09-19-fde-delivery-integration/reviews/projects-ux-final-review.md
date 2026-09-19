# 项目工作区最终短审查

结论：发现 1 项需要修复的组合缺口。仅只读审查当前冻结的 Page/projects.css、ProjectsWorkspace 与真实 kit/verification 组件及相关测试；没有改产品，没有重跑测试或操作 native UI。

## P2：切换三标签会丢失人工登记草稿

位置：`src/shared/features/verification/VerificationPanel.tsx:386`。

触发：在“验证与交接”打开“登记检查或客户验收”，填写登记人、角色、范围、依据等，切到“项目准备”或“交付方案”，再切回来。

依据：Page 用 PersistentSurface 给 inactive verification 面板传 visible=false；VerificationPanel 的 `manualOpen && visible && data` 会卸载 ManualForm。该表单在 :478 起将 person、role、scope、reference、issuer、basis 等全部存为本地 useState。返回后 ManualForm 重新挂载，内容重置。父层保留 ProjectVerification 本身不能保留已经卸载的子表单。

最小修复：让 ManualForm 在已展开且有数据时保持挂载；外层 tabpanel/PersistentSurface 已提供 hidden/inert，`disabled={!ready}` 和 act 的 visible 判断继续阻止隐藏时提交。补真实 VerificationPanel 的 hidden→visible 草稿保持定向用例。Page 当前 draft 测试注入的是 HandoffDraft 替身，不能覆盖这一真实表单的卸载条件。该问题已实时报告主控，由既有 owner 修复，本审查不接管实现。

## 其余检查

- 项目名称与工作说明状态归 ProjectEditor 所有，切 tab 不换 project key；prepare 面板不卸载。已访问 delivery/verification 才挂载入口，后续 PersistentSurface 传递可见性。
- `disabled || dirty` 从 Page 传到组合层；archived/dirty 显示明确原因。Kit 项目绑定/保存检查记录有 mutationBlockedReason 与 adapter 二次检查；Verification ready 与 act 同时门控，浏览/预览可独立保留。没有看到新 tab 自动触发全局工具启用或项目绑定。
- 新建项目入口和空客户起步路径存在，客户数据未就绪或读取失败时提交按钮禁用；只读切换项目不执行写入。
- CSS 保留 min-width/min-height:0；主详情改由 workspace tabpanel 承担 overflow:auto；760px 缩紧、620px 改纵向并限高目录区。源码未见确定的滚动归属冲突，但本次没有浏览器/native尺寸测量，因此不把静态核对写成窄窗视觉通过。
- Kit 隐藏会取消 preview 并清理临时 session，这是现有 stale-result 防护；本审查没有把该预览取消当作草稿缺陷。

证据层级：静态代码审查。测试通过状态来自 owner 的既有报告，未另行重复执行；原生视觉验收由主控负责。
