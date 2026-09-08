---
type: playbook
status: active
updated: 2026-08-12
review_on: 2026-09-12
authority: fy-agent/fyagent maintainers
source: current GitHub Discussions configuration and repository support contracts
---

# GitHub Discussions launch playbook

## Live seed discussions

- [Welcome to FyAgent Discussions / 欢迎来到 FyAgent 讨论区](https://github.com/fy-agent/fyagent/discussions/94) — Announcements, pinned globally
- [How to ask a question that is easier to solve / 怎样让问题更容易得到解决](https://github.com/fy-agent/fyagent/discussions/95) — Q&A, pinned in category
- [Which workflow should FyAgent support next? / 你希望 FyAgent 接下来支持哪个工作流？](https://github.com/fy-agent/fyagent/discussions/96) — Ideas
- [Share your FyAgent setup / 分享你的 FyAgent 配置](https://github.com/fy-agent/fyagent/discussions/97) — Show and tell

## Category contract

Keep the existing category names and slugs so the files under `.github/DISCUSSION_TEMPLATE/` continue to bind correctly. Improve the category emoji and description in GitHub settings instead of renaming the categories.

| Category | Emoji | Recommended description | Format |
| --- | --- | --- | --- |
| Announcements | 📣 | Project updates, release context, and community notices. / 项目动态、版本说明与社区公告。 | Announcement |
| Q&A | 🙏 | Installation, configuration, and usage help. Include version, OS, and related tool. / 安装、配置与使用帮助，请附版本、系统和相关工具。 | Question and answer |
| Ideas | 💡 | Explore a problem and possible direction before opening a scoped issue. / 在创建明确 Issue 前，先讨论问题与方向。 | Open discussion |
| Show and tell | 🙌 | Share real FyAgent setups, workflows, and lessons learned. / 分享真实配置、工作流与使用心得。 | Open discussion |
| General | 💬 | Conversations that do not fit the focused categories above. / 不属于上述分类的社区交流。 | Open discussion |
| Polls | 📊 | Maintainer-led community polls when a concrete choice needs feedback. / 仅用于维护者发起的明确选项调研。 | Poll |

## Seed discussions

### 1. Announcements — pin globally

**Title:** Welcome to FyAgent Discussions / 欢迎来到 FyAgent 讨论区

**Body:**

> FyAgent Discussions is the place for usage help, early product ideas, and real workflow sharing.
>
> FyAgent 讨论区用于使用帮助、早期产品想法和真实工作流分享。
>
> FyAgent is a local desktop tool for managing models, Skills, MCP servers, prompts, and memory files for supported AI applications.
>
> FyAgent 是一款本地桌面配置工具，用于管理受支持 AI 软件的模型、Skills、MCP、提示词和记忆文件。
>
> - Ask installation or configuration questions in **Q&A**.
> - Test an early proposal with the community in **Ideas**.
> - Share a setup that works for you in **Show and tell**.
> - Open an **Issue** for a reproducible defect or a clearly scoped task.
>
> 提问时请带上 FyAgent 版本、操作系统、相关工具和已经尝试过的步骤。提交日志前请移除凭据。
>
> Start with the [README](https://github.com/fy-agent/fyagent#readme), [manual](https://github.com/fy-agent/fyagent/tree/main/docs/user-manual), and [community guidelines](https://github.com/fy-agent/fyagent/blob/main/CODE_OF_CONDUCT.md).

### 2. Q&A — pin in category

**Title:** How to ask a question that is easier to solve / 怎样让问题更容易得到解决

**Body:**

> A useful question contains four things:
>
> 1. FyAgent version and operating system.
> 2. The related AI tool and provider type.
> 3. What you expected and what happened.
> 4. The documentation or troubleshooting steps you already tried.
>
> 一条容易解决的问题，通常包含：FyAgent 版本与系统、相关 AI 工具与供应商类型、预期结果与实际结果、已经查阅或尝试过的步骤。
>
> Use the **Q&A** form—the structured fields are there to save both the author and responders time. Reproducible software defects belong in [Bug Reports](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml).

### 3. Ideas

**Title:** Which workflow should FyAgent support next? / 你希望 FyAgent 接下来支持哪个工作流？

**Body:**

> Start with the task you are trying to complete, not a feature name.
>
> Tell us:
>
> - what you are trying to accomplish;
> - where the current workflow becomes repetitive, unclear, or fragile;
> - what result you need;
> - whether you can help validate or implement it.
>
> 请先说明你要完成什么、目前卡在哪里、需要什么结果，以及你是否愿意参与验证或实现。

### 4. Show and tell

**Title:** Share your FyAgent setup / 分享你的 FyAgent 配置

**Body:**

> Which FyAgent settings are you using, and which application do they affect?
>
> Share the setup, the problem it solves, and one lesson others can reuse. Screenshots, small diagrams, and configuration excerpts are welcome—remove credentials before posting.
>
> 欢迎分享你使用的模型、Skills、MCP 或提示词配置，以及一条别人可以复用的经验。可以附截图、流程图或配置片段；发布前请移除凭据。

## Operating rhythm

- Triage new Q&A posts into documentation gap, user configuration, reproducible defect, or unresolved investigation.
- Convert a mature Idea into an Issue only when the problem, outcome, and acceptance boundary are clear; link both directions.
- Add solved recurring questions to the manual or README FAQ instead of answering them from scratch indefinitely.
- Pin no more than two global discussions. Keep category pins focused on instructions or canonical references.
- Review unanswered Q&A and stale Ideas on a regular maintainer cadence; close the loop with a short status even when no implementation is planned.
