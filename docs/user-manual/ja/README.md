# FyAgent ユーザーマニュアル

このマニュアルは、実際の作業ごとに整理しています。初めての方は第 1 章から進み、目的が決まっている場合は下の入口から直接開いてください。

> 一部の旧スクリーンショットは、実際の FyAgent 画面で撮り直す必要があります。現在の操作名と本文を基準にし、画像内の `CC Switch` を現在の製品名と判断しないでください。

## 目的から探す

- FyAgent をまだ導入していない：[インストール](./1-getting-started/1.2-installation.md)を読む。
- まず 1 つのプロバイダーを動かしたい：[クイックスタート](./1-getting-started/1.4-quickstart.md)を進める。
- Claude Code などをインストール・更新したい：[ツールのインストール](./2-agent-tools/2.1-install.md)と[競合診断](./2-agent-tools/2.2-update-diagnose.md)を見る。
- エンドポイントやモデルをまとめて管理したい：[プロバイダーの追加](./3-providers/3.1-add.md)から始める。
- MCP、Prompts、Skills を使いたい：[拡張機能](#4-拡張機能)へ進む。
- WorkBuddy のモデル一覧を書き込みたい：[WorkBuddy のモデル設定](./4-extensions/4.6-workbuddy.md)を読む。
- リクエストが不安定、または使用量を確認したい：[プロキシと信頼性](#5-プロキシと信頼性)へ進む。
- 設定が反映されない：[よくある質問](./6-faq/6.2-questions.md)と[環境変数の競合](./6-faq/6.4-env-conflict.md)を確認する。

## 1. はじめに

- [1.1 FyAgent の紹介](./1-getting-started/1.1-introduction.md)
- [1.2 インストール](./1-getting-started/1.2-installation.md)
- [1.3 画面の見方](./1-getting-started/1.3-interface.md)
- [1.4 クイックスタート](./1-getting-started/1.4-quickstart.md)
- [1.5 個人設定](./1-getting-started/1.5-settings.md)

## 2. Agent ツール

- [2.1 ツールのインストールとバージョン確認](./2-agent-tools/2.1-install.md)
- [2.2 更新とインストール競合の診断](./2-agent-tools/2.2-update-diagnose.md)

## 3. プロバイダー

- [3.1 プロバイダーの追加](./3-providers/3.1-add.md)
- [3.2 プロバイダーの切り替え](./3-providers/3.2-switch.md)
- [3.3 プロバイダーの編集](./3-providers/3.3-edit.md)
- [3.4 並べ替え・複製・削除](./3-providers/3.4-sort-duplicate.md)
- [3.5 使用量クエリ](./3-providers/3.5-usage-query.md)
- [3.6 Claude Desktop](./3-providers/3.6-claude-desktop.md)

## 4. 拡張機能

- [4.1 MCP サーバー](./4-extensions/4.1-mcp.md)
- [4.2 Prompts](./4-extensions/4.2-prompts.md)
- [4.3 Skills](./4-extensions/4.3-skills.md)
- [4.4 セッション](./4-extensions/4.4-sessions.md)
- [4.5 ワークスペースとメモリー](./4-extensions/4.5-workspace.md)
- [4.6 WorkBuddy のモデル設定](./4-extensions/4.6-workbuddy.md)

## 5. プロキシと信頼性

- [5.1 ローカルプロキシ](./5-proxy/5.1-service.md)
- [5.2 アプリルーティング](./5-proxy/5.2-routing.md)
- [5.3 フェイルオーバー](./5-proxy/5.3-failover.md)
- [5.4 使用量統計](./5-proxy/5.4-usage.md)
- [5.5 モデルテスト](./5-proxy/5.5-model-test.md)

## 6. トラブルシューティング

- [6.1 設定ファイルと保存場所](./6-faq/6.1-config-files.md)
- [6.2 よくある質問](./6-faq/6.2-questions.md)
- [6.3 Deep Link インポート](./6-faq/6.3-deeplink.md)
- [6.4 環境変数の競合](./6-faq/6.4-env-conflict.md)

このマニュアルは現在のリポジトリの動作を説明します。インストーラー名、署名、信頼状態はリリースごとに変わるため、該当する [GitHub Release](https://github.com/fy-agent/fyagent/releases) と公開証拠を確認してください。
