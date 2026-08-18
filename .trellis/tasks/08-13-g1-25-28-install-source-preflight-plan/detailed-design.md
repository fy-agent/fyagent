# 详细设计（v1）：Install 四层契约与变更冻结

## 1. 数据模型（后端核心结构）

### 1.1 InstallContract 快照
```ts
type InstallContractState = {
  session_id: string;
  issue_link: ["#25", "#26", "#27", "#28"];
  catalog: SourceLayerState;
  package: IntegrityLayerState;
  environment: PreflightLayerState;
  plan: PlanLayerState;
  updated_at: string; // ISO8601
  snapshot_id: string;
}
```

### 1.2 SourceLayerState（#25）
```ts
type SourceLayerState = {
  source_state: "ok" | "warn" | "fail" | "unknown";
  official_origin: boolean;
  official_origin_url: string | null;
  package_source_kind: "official" | "partner" | "mirror" | "unknown";
  license_kind: "public_open_source" | "source_available" | "restricted_non_commercial" | "enterprise_only" | "unknown";
  distribution_allowed: boolean | null; // null 表示未确认
  source_evidence: {
    resolved_host: string;
    trust_anchor: string;
    checked_at: string;
    observed_from: string;
  };
}
```

### 1.3 IntegrityLayerState（#26）
```ts
type IntegrityLayerState = {
  integrity_state: "ok" | "warn" | "fail" | "unknown";
  computed_hash: {algorithm: "sha256" | "sha512", value: string, state: "ok" | "warn" | "fail" | "unknown"};
  signature_state: {valid: boolean | null, signer_id: string | null, cert_chain: string | null};
  revocation_state: {status: "active" | "revoked" | "unknown", evidence_id: string | null};
  integrity_summary: string;
  checked_at: string;
}
```

### 1.4 PreflightLayerState（#27）
```ts
type PreflightItem = {
  code: string;
  state: "pass" | "warn" | "fail" | "unknown";
  message: string;
  hint: string;
  checked_at: string;
  source: "agent" | "runtime" | "installer";
};

type PreflightLayerState = {
  preflight_state: "ok" | "warn" | "fail" | "unknown";
  checks: PreflightItem[];
  machine_facts: {
    os: string;
    arch: string;
    free_space_gb: number;
    user_scope: "user" | "admin" | "unknown";
    python_runtime: string;
  };
}
```

### 1.5 PlanLayerState（#28）
```ts
type PlanLayerState = {
  plan_snapshot_id: string;
  plan_summary: {
    agent_id: string;
    version: string;
    source_release_id: string;
    package_hash: string;
    actions: string[];
  };
  drift_rules: string[];
  snapshot_stale: boolean;
  drift_reasons: string[];
  refreshed_at: string;
}
```

## 2. 规则引擎

### 2.1 总体可继续条件
- `install_allowed = all_true([
  source.source_state == "ok" || source.source_state == "warn",
  integrity.integrity_state == "ok",
  preflight.preflight_state != "fail" && preflight.preflight_state != "unknown",
  !plan.snapshot_stale
])`
- `warn` 允许继续；`unknown/fail` 阻断。

### 2.2 不可静默变化触发
Plan snapshot 必须重算并阻断安装的触发条件：
- `source.package_source_kind` 变更
- `package.computed_hash.value` 变更
- `plan.plan_summary.version` 变更
- `plan.plan_summary.actions` 数组顺序或长度变更
- 任意 preflight 核心项 `fail -> pass` 或 `pass -> fail` 且用户未确认重试

### 2.3 记录策略
- 所有 `fail/unknown` 与 `snapshot_stale` 写入事件日志 `install_contract_events`。
- `snapshot_id` 需与 `plan.refresh`、`preflight.refresh` 同时更新，形成串联链。

## 3. 后端变更建议（非实现承诺）

### 3.1 兼容增量
- 在当前返回结构内追加字段，默认值 `unknown`，不破坏已有前端。
- 不增加外网请求；既有本机数据源采集优先，必要时仅读取本地缓存/元数据。

### 3.2 API 层
建议新增一个“安装快照聚合”接口（**独立于 Codex Desktop MSIX**）：
- `agent_install_get_contract(agentId)` 返回四层 `InstallContract`。
- `agent_install_reconfirm_plan(snapshotId)` 在 `snapshot_stale=true` 时重建
  `snapshot_id`。
- `agent_install_start_install({ snapshotId })` 是唯一执行入口。

### 3.3 迁移约束
- 低风险灰度：先支持字段写入，再逐步切到 UI 渲染。
- 对旧会话返回默认 `unknown`，强制展示“待确认”不是“可用”。

## 4. 前后端职责清单

| 项目 | 后端职责 | 前端职责 |
| --- | --- | --- |
| 获取来源证据 | 标准化字段 + 时间戳 | 展示来源与许可边界 |
| 完整性验证 | 计算 hash、签名、撤回记录 | 显示 `warn/unknown` 明细 |
| 环境预检 | 执行并返回错误码集合 | 转化为可读提示和修复步骤 |
| 安装计划冻结 | 生成 `snapshot_id` 与 drift_rules | 显示 stale 状态与重确认入口 |

## 5. 异常路径
- 前端缺字段：显示 `unknown` 并强制点击详情。
- 后端未返回 `snapshot_id`：视为 `stale`，允许进入观测但不允许安装。
- 接口超时：保留上次快照 + 标记 `source=stale`, 禁止提交 install。

## 6. 依赖顺序（执行）
1. #25 结构化 source metadata
2. #26 引入完整性字段字典
3. #27 统一 preflight code
4. #28 接口与 UI 重确认联动

