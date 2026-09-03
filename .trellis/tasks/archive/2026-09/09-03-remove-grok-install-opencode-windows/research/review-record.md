# 规划评审记录

评审日期：2026-09-03

## Round 1：范围与“是否删除干净”

检查 policy、backend、helper、frontend、docs、SPEC 与 tests 后，确认 Grok install并未删除；此前提交的真实意图是“退场非Grok installers、保留Grok唯一写能力”。

修正：任务从“删一个按钮”扩展为跨层 capability retirement，并明确保留只读探测，避免误删 Provider/配置消费者。

结果：通过。

## Round 2：OpenCode 官方来源与中国大陆目标

最初判断“macOS已用opencode.ai地址，因此方案无问题”不完整。继续追踪 resolver后发现：客户端仍硬依赖GitHub latest API，GitHub不可达时不会开始下载。

修正：stable alias成为source authority；GitHub version只能非阻断增强。后端使用locale-neutral route，不绑 `/zh`。加入客户端GitHub blocked测试和中国大陆HIL门禁。

结果：通过。

## Round 3：Windows installer与复用边界

核对官方route、Electron Builder config、当前stable EXE与FyAgent现有Windows链路：产物是签名NSIS EXE，适合Qoder/TRAE/WorkBuddy同类helper handoff，不适合Codex MSIX。

发现的关键细节：installer是i386 NSIS stub、ProductName为OpenCode、FileDescription为空。既有 verifier已有x64 host允许i386 installer stub的规则；不需要新框架，但身份policy不能错误要求FileDescription。

修正：任务强制复用现有EXE pipeline，并把 signer/installed target/path/registry放到当期Windows HIL中冻结。

结果：通过。

## Round 4：Grok更新是否应一起删除

用户要求的是一键安装退场，不是删除已安装用户的全部维护能力。直接删除update会扩大产品回退；原样保留又会留下 `update || installer`、bare npm和owner migration风险。

修正：采用中间但更严格的策略——保留唯一owner、已锚定的owner-preserving update；删除所有fresh install和fallback。无法证明锚定的owner暂时关闭更新。

结果：通过。

## Round 5：协议兼容、状态语义与支持声明

检查helper wire与Windows vendor handoff合同后，识别两个风险：

1. 删除Grok install后若复用其wire code给OpenCode，会产生协议歧义；
2. NSIS被拉起并不等于安装完成，不能在job success后立即标installed。

修正：Grok install wire values tombstone，OpenCode追加新值；沿用ShellExecute handoff success，安装状态由后续inventory确认。Windows ARM64与中国大陆可达性均保持HIL前不声明。

结果：通过。

## Round 6：推翻“移除 Grok 一键安装”，改为大陆 npm 默认安装

用户在完成网络调研后要求把方案并入本任务并实现到归档。新事实：

- 无 xAI 官方大陆镜像；原生 `x.ai` + GCS 不适合大陆默认。
- 官方 npm 包 `@xai-official/grok` 安装不访问原生下载主机。
- 腾讯云/华为云/npmmirror 均可装精确版本 `1.0.13`；npmmirror `@latest` 会指向 `0.1.4`。
- 因此默认一键安装改为官方 npm + 镜像链 + 应用内精确版本清单；禁止 `@latest`；原生安装降为显式动作。

OpenCode Windows x64 与 GitHub 非阻断解析保持不变。

结果：通过。任务进入实现。

