# 这次迭代一句话对齐

先读这篇。后面的 PRD、设计和测试表都是在讲同一件事。

## 我们要帮用户做成什么

家里已经买了 Grok / SuperGrok 的人，打开 FyAgent 后：

1. 能分清三种登录，不会走错门。
2. 扫码登录 SuperGrok 一次，就能把这颗脑子用到**所有现在该用、也能用的地方**：Claude Code、Claude Desktop、Codex，以及 WorkBuddy。
3. 必须 William 本人在 Windows 和 Mac mini 上各走一遍，才算做完。

名单上有名字，不等于已经能用。

## 登录有三条路，不要混

| 路 | 人怎么走 | 现在在哪 | 做成什么样 |
|---|---|---|---|
| 官方登录 | 终端里运行 `grok login` | 新界面 Agent 配置页 | 只帮你开门，**不说**已经登进去。 |
| SuperGrok 扫码 | 认证中心用官方网页登录 | 旧认证中心 | 路标指到认证中心。这把钥匙给后面几家工具共用。 |
| API 钥匙 | 自己填一把钥匙 | 新界面模型页 | 继续只填钥匙。不要出现 `grok login` 说明书。 |

Grok 官方登录做好了，**不会**自动做好 ChatGPT 登录。ChatGPT 是另一把钥匙，这轮不做。

FyAgent 不会翻 Grok 的秘密文件来假装已经登录。

## 这颗脑子用到哪里

上游已经能用 SuperGrok 扫码的，我们都要能用：

- **Claude Code**：新界面要能看见「先看、再改、再检查」。
- **Claude Desktop**：目录里没有单独一页，走现在的旧界面绑定，这轮要能亲测走通。
- **Codex**：和新界面预览绑在一起。今天新界面会拒绝 SuperGrok，这轮开一扇该开的门。

目录允许自己换模型的：

- **WorkBuddy**：可以。走它自己的「保存模型」，不是 Codex 那扇门。能用已经扫码的 SuperGrok 就不要再让人填第二把钥匙。
- **OpenCode**：上游若只是普通填钥匙，这轮不另做 SuperGrok 扫码。
- **Qoder**：明确不能配第三方模型。不做。
- **TRAE**：只能看，不能替它写模型。不做。

每一家分开改。改 Codex 失败了，不能说 Claude 或 WorkBuddy 也改好了。

## 怎么才算做完

- 程序自己的检查要过。
- William 用真实账号，在 **Windows 和 Mac mini** 上走完：三条登录，以及 SuperGrok 进 Claude Code、Claude Desktop、Codex、WorkBuddy。
- 少一家，或少一台电脑，都不算完。
- 密码不要写进仓库。

## 这次明确不做

- 不装、不升级 Grok。
- 不做还剩多少钱的看板。
- 不做 ChatGPT 那把登录钥匙。
- 不硬做 Qoder / TRAE 的模型写入。
- 不做「总门卫」。
- 不把官方登录显示成「已验证」。
- 不关 #42 / #43 整张工单。

## 谁做什么

- 窗口一：三条登录路标。
- 窗口二：SuperGrok 进 Claude Code、Claude Desktop、Codex。
- 窗口三：SuperGrok 进 WorkBuddy。
- 总控：对齐和两台电脑验收。
