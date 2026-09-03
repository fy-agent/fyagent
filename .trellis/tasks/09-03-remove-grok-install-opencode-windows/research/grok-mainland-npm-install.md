# Grok Build CLI 中国大陆一键安装调研

- 调研日期：2026-09-03
- FyAgent 基线：`dev/laiyongjie` @ `b3a297ab6eed4295c7ce486d0e509744731612f1`
- 官方稳定版本（xAI changelog）：Grok Build **1.0.13**（2026-08-28）
- 本文件只记录网络与官方分发证据。实现必须从官方 npm registry 重新读取当期 integrity，不得把研究时哈希当成未来版本准入。

## 1. 结论

截至 2026-09-03，xAI/SpaceXAI **没有**面向中国大陆网络的官方大陆镜像。

原生安装链只有两级：

1. `https://x.ai/cli`
2. `https://storage.googleapis.com/grok-build-public-artifacts/cli`

macOS `install.sh` 与 Windows `install.ps1` 都先访问 `x.ai`，失败后再退到 GCS。官方脚本没有可替换下载根地址的环境变量，不能无侵入切到国内镜像。

xAI 企业部署文档给出另一条**官方分发路径**：

```text
npm install -g @xai-official/grok
```

通过 npm 分发时，安装不需要访问 `x.ai` 或 `storage.googleapis.com`。这适合 FyAgent 做中国大陆安装适配。

最终产品决策：

> 中国大陆新安装默认使用 xAI 官方 npm 包；腾讯云 npm 镜像为第一源，华为云为第二源，npmmirror 为精确版本第三源，npm 官方源为最后源。版本和 SHA-512 不从镜像的 `latest` 获取，而由打进 FyAgent 签名应用的版本清单确定。禁止静默降级，不修改用户全局 `.npmrc`，保留现有 Native/npm 安装归属。

## 2. 信任边界

| 对象 | 角色 |
| --- | --- |
| `@xai-official/grok` | xAI 官方 npm 包 |
| 腾讯云 / 华为云 / npmmirror | 第三方镜像，只负责传输 |
| FyAgent 内置版本清单 | 版本与 SHA-512 的控制面；随已签名应用分发 |

镜像不能成为版本真相。npmmirror 在本次验证中可以把 `@latest` 错误指向 `0.1.4`，因此：

- 精确版本 `@1.0.13` 可用作兜底下载；
- 禁止用任何镜像解析 `@latest`。

## 3. 下载顺序

```text
1. https://mirrors.tencent.com/npm/
2. https://repo.huaweicloud.com/repository/npm/
3. https://registry.npmmirror.com/   # 只允许精确版本
4. https://registry.npmjs.org/
```

任一源缺失目标版本、哈希不符或安装失败，切换下一个。不能自动退回更旧目标版本。不执行 `npm config set registry`。

单次命令形态：

```text
GROK_NPM_REGISTRY=<selected>
npm install -g @xai-official/grok@<manifest-version>
  --registry=<same>
  [--allow-scripts=@xai-official/grok]   # 仅 npm major >= 12
```

npm 12 默认阻止未许可的依赖安装脚本。全局安装必须使用窄范围 `--allow-scripts=@xai-official/grok`，禁止 `--dangerously-allow-all-scripts`。旧 npm 若不认识该参数则不要附加。

## 4. 2026-09-03 官方 npm 验证

官方 registry 主包：

```text
@xai-official/grok@1.0.13
integrity = sha512-rBMEx/7ND5DaBRGwzi6fEyf4ZWy4yStPnZ38UaIM2smZzg4E0fieDfLKPK8eRF4l2Xe4+5kSdCAVop99+whG4A==
```

平台包（均 `1.0.13`）：

```text
@xai-official/grok-win32-x64
  sha512-IAoQ+fDQVUzwzl1gYK+CMka2cH4yN5nbcsyodyvYTvkxSPixDIzzWeRtrOESOCPlft6YYpvFfdqisJppPOHekA==
@xai-official/grok-win32-arm64
  sha512-J+USAEgMyy7TaDRir2HvS1rj8yG8HjjtewK0qxz1dl2E7jjdqMjaKuN6H6l563ASvXwNDx17ye+1fkwwpjyKdg==
@xai-official/grok-darwin-x64
  sha512-trI+fm4ZY/2skF05XLfCYCMQgk/NUj6jYVV1stL4jPafhYj41yDYxu0PHgLxEy6f8UtA8XCfZN084/KxgiYDKw==
@xai-official/grok-darwin-arm64
  sha512-Nctnwkzj550E512RZ+n+IUuhqGPZ2L7z/ZTIlZdVn0KqZHfzC58vIZPMBzdcOkoK42IrJlLD6GS7Eo6c1y+VGw==
```

隔离安装验证（腾讯云 / 华为云 / npmmirror）：精确版本存在、主包与平台包 SHA-512 与官方一致、`npm i -g` 成功、`grok --version` 为 `1.0.13`、构建标识 `5e9a58528b76`。

`@latest`：

| 源 | `@latest` |
| --- | --- |
| 腾讯云 | `1.0.13` |
| 华为云 | `1.0.13` |
| npmmirror | **错误指向 `0.1.4`** |

npm 包装层按 OS/CPU 依赖六个官方平台包；`postinstall` 只从本地已下载平台包解压二进制，安装期间不再访问 `x.ai` / GCS。

## 5. 当前代码必须修正的点

1. Windows helper 仍执行 `@xai-official/grok@latest`（`GROK_NPM_INSTALL_SPEC`）。npmmirror 会把 Windows 从 `1.0.13` 静默降到 `0.1.4`。
2. macOS 虽会冻结精确版本，但版本查询硬编码 `https://registry.npmjs.org/`。
3. 没有向 npm 传递 `--registry` 或 `GROK_NPM_REGISTRY`。
4. 多处仍生成 `npm i -g @xai-official/grok@latest`，需要集中为 `GrokNpmInstallPlan`。
5. 默认 `install`（owner 缺省）仍规划 `NativeFresh`，依赖 `x.ai` / GCS，不适合大陆默认。
6. native `update || installer` 与 npm bare PATH fallback 仍可能在失败后创建新安装。

## 6. 产品边界（安装成功 ≠ 大陆可完整运行）

国内 npm 镜像只解决安装包下载。xAI 运行仍依赖：

```text
cli-chat-proxy.grok.com
auth.x.ai
```

产品文案必须是：

> 支持通过中国大陆 npm 镜像安装 Grok Build CLI；Grok 登录与在线推理服务的可用性仍取决于 xAI 服务在用户网络中的可达情况。

## 7. 不采用

- 复制/改写官方 `install.sh` / `install.ps1` 并自建二进制 CDN。
- GitHub Release / 社区代理作为产品默认。
- 永久修改用户全局 npm registry。
- 把镜像 `@latest` 当版本真相。
- 静默把已有 native 安装迁到 npm。
