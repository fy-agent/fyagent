Active task: /Users/serendipity/fyagent/.trellis/tasks/09-22-session-cross-device-recovery
继续仅修两个UX文件，保持既有设计，不重写整份。浏览器已验证导出拦截与重复导入默认跳过。仍有以下真实逻辑问题：
1. markUserConfirmed 和 applyRemap 把 status 设normal，可让原unsupported会话绕过handleResumeInTarget门槛！将 capability 状态与 userConfirmed、workspace 状态拆开，用户确认/目录映射绝不可提升恢复能力。handleResumeInTarget还必须拦截任一isIndeterminate轮次。
2. copyContinuationCommand直接用源originalId且所有provider拼 --resume，不是真正目标ID与真实命令，删除实际命令复制功能或改成“查看启动说明（模拟）”，无已验证目标ID则禁用。不要生成可执行伪命令。
3. 导出按钮现在下载伪 fyagent.session.v1（turns与空assistant、cwd_hint），与技术方案尚未冻结不一致。此HTML只做交互演示，不要下载假标准包；改为“预览导出内容（演示）”展示纯文本问答，无final时省略而非填空。明确不是可导入生产包。
4. 未判定消息卡片不应仍标“最终答复 Final Answer”并显示原始进度日志；展示“答复待判定”与简短原因，不展示未过滤执行内容。
5. ux-spec 里从API空列表推导“原生界面不可见”太强，改成“历史查询为空，桌面UI展示未验证”；最新本机模拟请求已证明重启后四条历史进入下轮请求，但不是真实模型续聊。
6. 删除原型中堆满打勾的自我验收大抽屉，用简短演示说明，减少流程词。保留状态切换为演示辅助。
完成后返回实际自检、文件。