Active task: /Users/serendipity/fyagent/.trellis/tasks/09-22-session-cross-device-recovery
你是本轮 A2A 执行者，GPT-6 主协调担任总产品经理/架构师。用户授权今晚 Session 跨设备恢复的实施与准备，指定 Antigravity 设计、Cursor 技术方案和测试、Grok 独立去冗余评审。
基线仓库 /Users/serendipity/fyagent，origin https://github.com/fy-agent/fyagent.git，分支 codex/frontend-interaction-v3-1-20260826，HEAD 60d699fa00b3f275dcda556afc74a21eaa05379d；工作区有大量已有界面 dirty changes，全部保护。读取 AGENTS.md 和 .trellis/workflow.md；本包仅方案/原型/隔离验证，禁止改生产源码、提交、发布、删除或写真实会话库。
必读 research/session-recovery-20260921/恢复方案.md、同目录 cli-formats.md、本地 JSON 证据，以及 .trellis/tasks/09-21-remove-client-projects/research/session-direction.md、session-feasibility.md。根据责任范围读取实际源码并引用位置。以最新用户明确方向为准：Session 替代记忆一级入口；每轮用户原文+最终答复原文，不摘要，不迁移工具调用/结果、进度、思考、附件及文件；不删除旧记忆文件；目标对应软件原生续聊、ID可变；查看/复制不算恢复。七软件是产品范围，先验证 Codex/OpenCode 是实施顺序而非永久缩小范围。
验收区分：包导入、原生读回、打开、重启后读回、下一轮续聊、Mac/Windows双向。未运行须明确未验证。未知能力不假定不可能，不写假支持。
A2A：主协调通过独立文件交付与真实 session/process 回执收件；一文件一 writer；禁止额外平台/常驻队列。你仅写自己的 deliverables 文件（指定如下），临时文件只用本 task/evidence/你的角色 子目录。产物必须写盘，自检后返回路径、执行模型、阻塞、下一依赖。
Jev 可用于有证据的有限取舍，不用于代替测试/授权。如本端没有 Jev MCP，将请求写 decisions/<角色>-request.json：decision, candidates(2-6个id/description), evidence(事实/来源/不确定性), priorities, requirements；主协调回传 response。无必要不强制调用。不要等待 Jev 阻止独立工作；不要配置新凭据。
所有终端命令遵循 ~/.codex/RTK.md。遇到缺登录/模型失败/范围冲突，留下真实错误，停止相关动作，不静默换模型。不要要求用户重复确认已经授权的文档工作。

你是 Cursor 的高级技术设计执行者。交付 deliverables/technical-design.md，精确到 Rust/TS DTO、命令边界、final-only提取、包格式版本、大小限制、稳定去重身份、目标本地恢复映射、崩溃窗口、目录元数据映射、版本检测、错误语义、重试与回退。比较直接JSON文件+小型恢复receipt与新增通用会话数据库/任务引擎，给最小充分方案。逐一说明七provider的真实证据等级与缺口；引用源码，不把历史文档当已实现。给可分工文件范围与实施依赖。不得改生产代码。