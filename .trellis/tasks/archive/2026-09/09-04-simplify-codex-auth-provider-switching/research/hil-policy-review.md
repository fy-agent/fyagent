# HIL 门控策略复核

> 评审日期：2026-09-04

## 1. 当前问题

Codex consumer使用编译期常量把生产file projection永久关闭，理由是等待matching-host HIL。该策略混淆了：

1. 某平台/版本是否做过人工兼容性验证；
2. store、身份、权限、原子性、回填、revision、readback、owner和补偿是否满足安全合约。

HIL可以发现兼容问题，但不能单独证明第二类不变量；反过来，缺少一次HIL记录也不应使已满足第一方合约和自动化证据的能力永久不可达。

## 2. 替代策略

生产capability由机器可判定事实共同决定：

- 固定第一方contract支持目标schema/default；
- effective store为unset/default-file或explicit-file；
- policy与identity匹配；
- token material完整；
- refresh lineage单一owner；
- Provider回填在auth覆盖前成功；
- serialized mutation + expected revisions；
- atomic write + owner-only permissions；
- auth/config/Provider/identity readback；
- revision-aware compensation；
- certainty丢失时返回closed partial/recovery状态。

全部成立才广告action和completed结果。

## 3. HIL保留角色

HIL继续用于：

- 新Codex版本兼容性抽查；
- macOS/Windows CLI/Desktop真实pickup；
- restart UX与长期自动refresh；
- 发布后回归/支持矩阵。

HIL不再：

- 控制生产常量；
- 成为有效file projection的必选验收项；
- 替代fault-injection/readback；
- 在未执行时把能力统一报成`native_projection_unavailable`。

## 4. 范围边界

本复核只改变Codex。Grok Build有独立auth lock、credential precedence和hot reload合约；OpenCode已有生产writer但对runtime pickup保持保守。不能因为都提到HIL而机械共享门控或同时开放。
