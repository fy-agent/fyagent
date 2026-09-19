use super::types::*;

pub(crate) fn preview(snapshot: VerificationSnapshot) -> VerificationResult<HandoffPreview> {
    // Only the public view enters serialization. Native dependencies, secret
    // epochs, arbitrary notes, raw responses and recovery capabilities do not.
    let json = serde_json::to_string_pretty(&snapshot).map_err(|_| "export_failed")?;
    let mut markdown = format!(
        "# 项目交接\n\n项目：{}\n\n生成时间：{}\n\n",
        snapshot.project_id, snapshot.checked_at
    );
    if !snapshot.available {
        markdown.push_str("项目当前状态不可读取；历史记录不能作为当前通过。\n\n");
    }
    for stage in [
        Stage::ConfigurationSaved,
        Stage::AuthenticationAvailable,
        Stage::ToolCallable,
        Stage::SamplePassed,
        Stage::CustomerAccepted,
    ] {
        let label = match stage {
            Stage::ConfigurationSaved => "配置已保存",
            Stage::AuthenticationAvailable => "认证可用",
            Stage::ToolCallable => "工具可调用",
            Stage::SamplePassed => "样本通过",
            Stage::CustomerAccepted => "客户已验收",
        };
        markdown.push_str(&format!("## {label}\n\n"));
        let records: Vec<_> = snapshot
            .evidence
            .iter()
            .filter(|e| e.stage == stage)
            .collect();
        if records.is_empty() {
            markdown.push_str("未检查\n\n");
        }
        for e in records {
            markdown.push_str(&format!(
                "- {} / {}；来源 {}；检查时间 {}；检查器 {} v{}；应用 {}；项目版本 {}\n",
                match e.outcome {
                    Outcome::Passed => "通过",
                    Outcome::Failed => "失败",
                    Outcome::Unknown => "未确认",
                    Outcome::Unsupported => "暂不支持",
                    Outcome::Cancelled => "已取消",
                },
                match e.validity {
                    Validity::Current => "当前有效",
                    Validity::Stale => "待复核",
                    Validity::Revoked => "已撤销",
                    Validity::Unverifiable => "无法核对当前状态",
                },
                match e.source_class {
                    SourceClass::NativeLocal => "本机检查",
                    SourceClass::NativeRemote => "服务请求",
                    SourceClass::LocalFixture => "本机模拟样本",
                    SourceClass::ManualRecord => "人工登记",
                },
                e.observed_at,
                e.checker_id,
                e.checker_version,
                e.app_version,
                e.project_revision
            ));
            if let Some(fixture) = e.fixture {
                markdown.push_str(&format!(
                    "  样例：{}\n",
                    match fixture {
                        KitFixture::Baseline => "正常周报",
                        KitFixture::MissingField => "缺少必填字段",
                    }
                ));
            }
            if let Some(sample) = &e.sample {
                markdown.push_str(&format!(
                    "  业务输入：{}；符合样例预期：{}；本机合成输入摘要：{}\n",
                    if sample.code == SampleCode::Ok {
                        "通过"
                    } else {
                        "未通过"
                    },
                    sample.matches_expectation,
                    sample.input_digest
                ));
            }
            if let Some(kit) = &e.kit {
                markdown.push_str(&format!(
                    "  包：{} / {} / {}\n",
                    kit.kit_id, kit.kit_version, kit.manifest_digest
                ));
            }
            if let Some(m) = &e.manual {
                markdown.push_str(&format!(
                    "  人工登记：{}（{}）；范围：{}\n",
                    m.person, m.role, m.scope
                ));
                if let Some(b) = &m.external_basis {
                    markdown.push_str(&format!("  外部依据：{}（{}）\n", b.reference, b.issuer));
                }
            }
            markdown.push_str(&format!(
                "  原因：{}；依据记录：{}\n",
                e.reason_code,
                e.basis_evidence_ids.join(", ")
            ));
        }
        markdown.push('\n');
    }
    markdown.push_str("## 待办与接手人\n\n");
    if snapshot.handoff.items.is_empty() {
        markdown.push_str("未登记待办；不代表没有未完成事项。\n");
    }
    for item in &snapshot.handoff.items {
        markdown.push_str(&format!(
            "- [{}] {}；责任人：{}\n",
            if item.completed { "x" } else { " " },
            item.title,
            item.owner.as_deref().unwrap_or("未指定")
        ));
    }
    markdown.push_str(&format!("\n## 回退\n\n{}\n\n文件恢复只覆盖受支持的本机文件；不会撤销账户授权、数据库或远端业务。外部改动后应重新检查恢复条件。\n",match snapshot.handoff.rollback.unwrap_or(Rollback::NotAvailable) {Rollback::ReviewConfiguration=>"检查配置后手工回退",Rollback::GuardedFileRecovery=>"检查文件恢复条件",Rollback::ManualOnly=>"由接手人手工处理",Rollback::NotAvailable=>"尚无回退办法"}));
    markdown.push_str(
        "\n本机样本结果不证明外部服务已连接；人工登记不代表 FyAgent 已核验签署人身份。\n",
    );
    Ok(HandoffPreview {
        snapshot,
        json,
        markdown,
    })
}
