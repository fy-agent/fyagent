# omp YAML 局部编辑选择

建议使用 **`yaml-edit = "=0.3.3"`**，以原 CST 修改选定叶子，保留其他节点。官方仓库 [`jelmer/yaml-edit`](https://github.com/jelmer/yaml-edit)，固定版本提交 [`ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2`](https://github.com/jelmer/yaml-edit/tree/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2)。这是已发布、有 Windows CI 的现成编辑库；仍处于 0.x，不能把库名当成所有 YAML 情形已验收的保证。本轮只查官方资料及源码，未安装依赖或改 FyAgent 源码。

它用 rowan CST 保留注释、空白、引号、顺序、未识别业务字段。`Mapping::set` 对已有键调用 entry 的局部 value 替换，**不会重建整个 provider**；替换实现明确搬回原值行注释，并保留原 anchor。[官方说明](https://docs.rs/yaml-edit/latest/yaml_edit/) · [set 实现 L540–546](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/src/nodes/mapping/mod.rs#L540-L546) · [注释保留 L591–639](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/src/nodes/mapping/entry.rs#L591-L639)

## 最小 API 示例

以下只更新**已存在、直接以 mapping 表示**的 provider 的 `baseUrl`；错误时交回调用者，不写文件。API 已与上述固定版本源码逐项核对，本轮未编译执行。

```rust
use std::error::Error;
use std::str::FromStr;
use yaml_edit::YamlFile;

fn patch_existing_base_url(
    input: &str,
    provider_id: &str,
    new_url: &str,
) -> Result<String, Box<dyn Error>> {
    let file = YamlFile::from_str(input)?;
    // 保守门控：编辑前必须原样往返。
    if file.to_string() != input {
        return Err("YAML round-trip changed the source".into());
    }
    let mut docs = file.documents();
    let doc = docs.next().ok_or("missing YAML document")?;
    if docs.next().is_some() {
        return Err("OMP configuration must contain one document".into());
    }
    let root = doc.as_mapping().ok_or("root is not a mapping")?;
    let providers = root.get_mapping("providers").ok_or("missing providers mapping")?;
    let provider = providers.get_mapping(provider_id).ok_or("missing direct provider mapping")?;
    provider.set("baseUrl", new_url);
    let output = file.to_string();
    YamlFile::from_str(&output)?;
    Ok(output)
}
```

依据：[`YamlFile::from_str`](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/src/yaml.rs#L672-L687) 发现语法错误会返回 Err；[`Mapping::get_mapping`](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/src/nodes/mapping/mod.rs#L108-L115) 返回原树中的 mapping；set 的共享内部可变性让修改直接体现到 `file.to_string()`。动态 provider/model 名称应作为键传入，不拼接含点号的 path 字符串。

## Windows、Rust 与适用边界

- 包声明 Rust **1.70 / edition 2021**，Apache-2.0，核心依赖 rowan；这是声明的 MSRV，本轮未复验 1.70 编译。固定提交的 [`windows-latest` 官方 CI job](https://github.com/jelmer/yaml-edit/actions/runs/39642857615/job/119370642911) 已成功，工作流包含 build、all-features、无默认 features、测试和 doctest。[Cargo.toml](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/Cargo.toml#L1-L23) · [工作流](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/.github/workflows/rust.yml)。本次 shell 的 `rustc --version` 报没有默认 toolchain；未改变 rustup，项目应继续使用主线程已选定的实际工具链。
- 使用 **YamlFile** 保存 stream 级注释、`---` 外注释和 directives。`Document::from_str` 会拒绝它不能往返的 stream 内容，`from_str_ignoring_comments` 则明确丢弃；后者不适合配置保存。[Document 规则](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/src/nodes/document.rs#L625-L644)
- `set` 替换的是传入的那个 value：若传入整个 provider/模型数组，旧子树仍会被替换。应遍历匹配模型 ID 后改叶子、追加新节点；不要从表单少数字段重造 provider。未触及的未知字段保持在树中。
- 别把 alias/merge 的解析结果重新展开写回。直接 mapping 查找遇到 alias 可拒绝；继承于 `<<:` 的值、重复键、anchored 目标需要单独明确策略。更新被其他 alias 引用的 anchor 值可能影响多处语义，CST 保留不能消除这一影响。
- 正式接入需用代表性 fixtures 检查：头部/行尾注释、未知字段、flow mapping、带斜杠/点号键、CRLF、block scalar、anchors/aliases、重复键、第二次保存幂等。新增/删除结构可能调整局部空白，不能承诺所有修改逐字节不变。官方 CRLF 编辑 invariants 已纳入测试，固定版本已知失败清单仅含说明注释。[CRLF 测试](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/tests/edit_invariants.rs) · [清单](https://github.com/jelmer/yaml-edit/blob/ab1afd283f4a78c3fa3ab04cbf3ba6d16c8048e2/tests/edit_invariants_crlf_known_failures.txt)
- CST 输出仍需独立语义验证、CAS、备份及原子写；用现成 omp 隔离 CLI 验证最终 YAML。这个库不提供 FyAgent 的事务、凭据隔离或产品验收。

## 为什么不选 saphyr 做这一层

`saphyr 0.2.2` 是纯 Rust YAML 解析与 emitter 库，公开值模型保留部分 scalar style / tags，但没有 comments/whitespace CST 节点，常规流程为 load → Yaml 数据 → emitter。它的官方文档还明确部分 core tags 默认不保留。可作语义解析工具；没有证据支持用其普通 emitter 实现原文件注释/空白的无损局部编辑，本任务选择 yaml-edit。[saphyr 官方 API](https://docs.rs/saphyr/latest/saphyr/) · [Yaml 值模型](https://docs.rs/saphyr/latest/saphyr/enum.Yaml.html)

## 本次实际依赖探针发现的边界

`yaml-edit 0.3.3` 向既有 flow mapping 插入空的 `Mapping::new()`、随后取得该新节点并追加字段时，独立探针复现了 `{}` 后紧跟字段的无效 YAML。实现采用先填充脱离文档的新 mapping/sequence，再一次插入；现有节点继续原位修改。配置服务对输出继续做语义解析，并以 `new_collections_preserve_valid_flow_yaml_and_existing_provider_fields` 覆盖空根、空 providers、已有 flow provider/sequence 和默认角色。这是本次具体使用方式的修正，不承诺所有 YAML 构造都能任意无损修改。