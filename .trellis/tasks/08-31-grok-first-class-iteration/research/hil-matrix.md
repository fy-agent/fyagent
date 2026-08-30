# 亲测表

先读 [../summary.md](../summary.md)。

做完的意思：William 在 **Windows** 和 **Mac mini** 上把下表都走一遍。结果可以写在这里，**不要**写密码、验证码、邮箱、账号名。

能写的只有：屏幕上写了什么、有没有出现「已验证/已登录」、有没有先给你看要改什么、检查有没有通过、失败时指去了哪扇门。

## 程序自己先查

| 编号 | 查什么 | 谁锁 |
|---|---|---|
| AT1 | Grok 点登录后是「已交给官方认证入口」，没有「认证结果已验证」 | 登录窗口 |
| AT2 | 字里有 `grok login`，没有叫扫码去终端 | 登录窗口 |
| AT3 | Claude 原来能验证的路还在 | 登录窗口 |
| AT4 | Codex 认证区没有登录按钮，指向认证中心 | 登录窗口 |
| AT5 | 空的 Grok 模型草稿，没动手就不报错；没改草稿就标没碰 | 登录窗口 / #141 B7 |
| AT6 | 没账号时预览仍拒绝 SuperGrok；有账号时可以预览，单子里没有钥匙 | 投放窗口 |
| AT7 | 新界面没账号时指向认证中心，不搬旧表单 | 投放窗口 |
| AT8 | Claude Code 失败不谎报 Codex / WorkBuddy 已改好 | 投放窗口 |
| AT9 | WorkBuddy 预览走自己的保存，不走 Codex upsert；单子里没有刷新令牌 | WorkBuddy 窗口 |

## 两台电脑都要走

| 编号 | 人怎么走 | Windows | Mac |
|---|---|---|---|
| H1 | 新界面 Grok → 登录 → 终端出现 `grok login` → 软件仍不说已登录 | | |
| H2 | 同一页退出 → 终端 `grok logout` → 仍不说已验证 | | |
| H3 | 认证中心扫码登录 SuperGrok 成功；过期指回认证中心，不是 `grok login` | | |
| H4 | 模型页 Grok 只填 API 钥匙，没有 `grok login` 说明书 | | |
| H5 | 已登录 SuperGrok → Claude Code → 先看 → 确认 → 检查通过 | | |
| H6 | 已登录 SuperGrok → Claude Desktop（旧界面即可）→ 先看 → 确认 → 检查通过 | | |
| H7 | 已登录 SuperGrok → Codex → 先看 → 确认 → 检查通过 | | |
| H8 | 已登录 SuperGrok → WorkBuddy → 先看 → 确认 → 检查通过 | | |
| H9 | 故意取消或失败一家，不说别人的工具也被改好了 | | |

空一格就不能说做完。
