# 用例 — SuperGrok → WorkBuddy

双机勾选表在父任务 `research/hil-matrix.md`。

## UC-W1 没账号不假装写入

- 对应 AT9 的「没账号」半边
- 人：没登录 SuperGrok，打开 WorkBuddy 模型
- 期望：指向认证中心；不生成一笔假装成功的 WorkBuddy 保存

## UC-W2 已登录能保存并回读

- 对应 H8
- 人：已登录 SuperGrok → WorkBuddy → 先看 → 确认 → 检查
- 期望：预览 operation 是 `workbuddy_models_save`；用已登录账号拉模型名单；单子里没有刷新令牌；回读看得到模型
- 若文件格式仍要一把钥匙：预览说清楚要什么；**不要**把 OAuth 刷新令牌写进 `models.json`

## UC-W3 失败不连坐

- 对应 H9
- 人：故意取消 WorkBuddy 预览
- 期望：不说 Claude / Codex 已改好

## 本窗口不做

登录三条路。Claude / Desktop / Codex 绑定。Qoder / TRAE。把 WorkBuddy 改成 Provider。
