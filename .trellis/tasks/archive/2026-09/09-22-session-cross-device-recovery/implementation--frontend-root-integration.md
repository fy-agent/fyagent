> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

AGY review-fixes退出0后root最小整合：空结果只说明无法确认，不猜测写入未发生，不建议新副本重试；恢复记录视图probe按选中receipt.targetProviderId选取，不能借用当前source列表旧选择的provider；目标打开只有已native readback verified（及更高已证实阶段）才启用，unknown/pending/failed不靠按钮提升。QA现可验收，前端写权已释放。
QA final发现900×600主导航末项底边溢出。root在AGY和QA退出后把auxiliary的内联间距归回shell.css，并在≤640高缩小组间空白；不缩控件点击面积、不放宽测试。待定向shell复测。
