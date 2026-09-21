> 历史准备材料（superseded）：最终实现合同见 `.trellis/spec/backend/session-migration.md`，当前能力与验收见 `implementation/delivery-readiness.md`。下文的 src/v2、Codex inject-only 及早期 provider 缺口判断不作为当前产品事实。

# 架构裁定（协调版）

## 产品合同
继承 prd.md。恢复精简问答，目标使用对应软件和本机账号。不得以完整事件备份、摘要、复制替代。

## 最小架构
1. 复用七来源扫描器；从原始事件做来源专用 final-only 投影，禁止从 role/content/ts 展示DTO导出。真实用户消息与运行时注入须分开。
2. 单个有版本的 JSON 包保存保真问答与最低必要身份、顺序、完成状态；不打包原生存储，不增加压缩归档、云账号、同步服务或插件框架。正文中的路径、代码保留；目标目录只用于目标侧元数据。
3. 包内容以稳定来源身份+规范内容摘要去重。目标侧生成新原生ID，复用现有SQLite小型 receipt 表记录映射和恢复阶段，不存正文。目标已写但 receipt 丢失时不得盲重试；先对账。明确另存才建副本，不覆盖、不自动合并两端新增历史。
4. 每个软件/版本能力独立标记。包导入、原生读回、打开、重启后读回、续聊、双向OS均分证，不把用户自报当自动验证。
5. 协作通过本任务目录、独立writer与实际session回执，Jev仅辅助有限语义决策，不进入产品恢复运行时。

## GPT-6 负责的关键验证
Codex 0.154.0隔离 CODEX_HOME：公开注入成功；四条原文持久化；重启resume成功；向本机模拟端点发下一轮请求时，四条原文按正确角色、顺序和final_answer phase出现。没有调用真实模型、没有使用真实历史。
重要缺口：paginated read/items/turns列表为空；legacy亦未解决。下轮请求可见和原生UI历史可见是两个验收项。真实回复、桌面UI、Mac/Windows双向仍未验证，不能标Codex完全支持。

## 决策
Jev 在足够证据下建议逐版本能力门槛、公共部分并行推进（decisions/restore-gate-response.json）。该建议与架构判断一致，但不构成正确性或验收证明。

## 整合状态
本文件为协调约束。详细DTO/接口见交付 technical-design.md；测试见 test-plan.md/test-cases.json。独立Grok评审意见需逐条裁定，作者产物完成不自动等于验收通过。

## 回执存储裁定
初稿sidecar优先建议已 superseded：既有SQLite可复用事务和唯一约束处理并发去重，独立文件反而要自建锁与索引。采用现有SQLite小表；Jev给出同向建议，但不构成测试证明。外部写入仍不在本地事务内，须保留needs_reconciliation状态。具体迁移版本号实施时读取。表要排除跨设备同步，普通备份恢复也必须验证设备绑定，不能只改WebDAV。

## 技术方案首稿问题（已在集成稿修订；生产验证未完成）
- Codex磁盘已有phase字段，本轮合成持久化证明它存在；不能把现有reader没读当格式不支持。位置只能是线索，不能默认放行final-only导出。
- migration_key包含source_session_id却宣称目标ID改变后仍相等，逻辑不成立。来源身份、纯内容摘要和目标映射必须分开；对账不能拿目标新ID重算来源key。
- 单migration_key主键无法保留SaveAsNew多副本映射；恢复尝试ID与默认导入唯一slot需要分开。
- 不能将客户端自报continued冒充客观续聊成功；包、原生写入、原生可见、打开、重启、真实续聊分别记录。
- “重试=新建另一份”与幂等冲突；已知无副作用可重试，不确定则对账，另存副本是独立显式操作。

语义快照由 originId + contentDigest 生成；不使用原始包字节 hash 去重。缺少可信原生身份时为来源实例保存独立 UUID。默认 slot 加入目标 provider/store/设备绑定；所有恢复请求包含 requestId，另存副本同请求重试也幂等。详见 technical-design.md §4/§8 与参考 SQLite 合同实验。
