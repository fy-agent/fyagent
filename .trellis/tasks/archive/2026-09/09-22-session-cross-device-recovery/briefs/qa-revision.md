Active task: /Users/serendipity/fyagent/.trellis/tasks/09-22-session-cross-device-recovery
仅定向更新你负责的 deliverables/test-plan.md 与 test-cases.json；不要重研全仓，不改生产代码。读 grok47-final-review.md、design.md、briefs/architecture-revision.md。技术方案作者正在同一方向修订，旧技术正文若冲突以协调返工指令为准，别依赖positional放行。
必须统一：
- 所有歧义final（含最后轮）阻止导出；真正无assistant的未完成用户输入可以保留，连续user和中断后再问不是天然坏数据。不要要求只允许末轮缺assistant。角色有序序列不能通过硬凑turn篡改。
- 删除QA-IMP-003的原地自动追加预期：同源较新包和分叉均不自动覆盖/合并，显式另存新原生ID。
- 本地回执用现有SQLite小表，测试INSERT/UPDATE/事务边界和唯一slot，而非sidecar atomic rename。导出JSON文件仍用atomic_write。
- 目标ID更改后对账不能直接重算带sourceID的key。区分纯正文digest、来源identity、target mapping；加入同内容不同源、无源ID、空assistant vs缺assistant、再导出来源映射丢失的保守行为。
- SaveAsNew每次有独立请求/instance，同请求重放幂等，旧副本映射不丢；一个migration_key不能覆盖多副本。普通备份搬到另一机器后不能承认本机已恢复；目标store身份不同回执无效。
- 原生操作非0/超时仍可能有副作用，0命中也不能证明未写；needs_reconciliation时不再写，只有证明无副作用才允许同尝试安全重试。
- 拆native_written/readback/opened/restart/request-context/real-continuation；user_attestation不推进系统验证stage。用户确认/改目录不能解锁capability，原型此漏洞已修。
- 错误码/限制最后对照新technical-design；如果新稿尚未完成，保留明确映射表，不让两套码假装一致；具体应落实为一个命名体系。保留JSON深度/unknown字段/大小测试必要边界。
- 35条旧用例全部not_run，新增也不得标passed。本轮另有 evidence/prototype-contract-test-result.json 六条隔离DOM测试全过，与生产用例不同；Codex请求捕获是模拟端点，不是模型真续聊。
把机械对齐即可的措辞改掉，这是行为修正。报告最终条数、变更ID、语法自检。