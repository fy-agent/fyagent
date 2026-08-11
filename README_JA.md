<div align="center">
  <img src="assets/fyagent.png" width="128" alt="FyAgent アイコン">
  <h1>FyAgent</h1>
  <p>日常的に使う AI コーディングツールの設定を、ひとつのデスクトップアプリへ。</p>
  <p><a href="README.md">English</a> · <a href="README_ZH.md">简体中文</a></p>
</div>

FyAgent は、プロバイダー、拡張機能、プロキシルーティング、利用履歴をまとめて管理します。Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes に対応しており、モデルやエンドポイントを変えるたびに複数の設定ファイルを手で編集する必要がありません。

> **リリース状況:** FyAgent は現在も継続的に開発されています。更新前に大切な設定をバックアップし、インストール前に各リリースの信頼情報を確認してください。

## できること

- 組み込みプリセットまたは互換 API からプロバイダーを追加し、設定ファイルを書き直さずに切り替える。
- MCP サーバー、再利用する Prompts、Skills を対応ツール間で管理する。
- ローカルプロキシでリクエストを中継し、フェイルオーバールールとモデル疎通を確認する。
- token 使用量と推定コストをひとつの画面で確認する。
- ツールごとの履歴フォルダーを探さず、セッションやワークスペースから作業を再開する。
- 秘密情報を手元の端末に置いたまま、設定をバックアップ・同期する。

作業データは既定でローカルの `~/.fyagent` に保存されます。SQLite とアトミック書き込みを使い、設定更新の途中で壊れたファイルを残しにくくしています。`fyagent://` から設定を取り込む場合も、書き込む前に変更内容を表示します。

## ダウンロードとインストール

[GitHub Releases](https://github.com/fy-agent/fyagent/releases) から、お使いの環境に合うファイルをダウンロードしてください。ファイル名は次の形式です。

- macOS: `FyAgent-X.Y.Z-macOS.dmg`、`FyAgent-X.Y.Z-macOS.zip`
- Windows: `FyAgent-X.Y.Z-Windows-x64-setup.exe`、`FyAgent-X.Y.Z-Windows-arm64-setup.exe`
- Linux x64: `FyAgent-X.Y.Z-Linux-x86_64.AppImage`、`FyAgent-X.Y.Z-Linux-x86_64.deb`、`FyAgent-X.Y.Z-Linux-x86_64.rpm`
- Linux arm64: `FyAgent-X.Y.Z-Linux-arm64.AppImage`、`FyAgent-X.Y.Z-Linux-arm64.deb`、`FyAgent-X.Y.Z-Linux-arm64.rpm`

Windows 版は NSIS セットアップのみで、MSI とポータブル ZIP は現在の配布対象ではありません。macOS 版は ad-hoc 署名で、Apple Developer ID では署名されておらず、公証も受けていません。Flatpak はセルフビルド用で、公式リリース成果物ではありません。

インストール前にリリースノートを読み、公開されたチェックサム、`signing-status.json`、ビルド証明を確認してください。`NotSigned` は署名状態を示すだけで、安全性の証明ではありません。各 OS の手順は[インストールガイド](docs/user-manual/ja/1-getting-started/1.2-installation.md)にまとめています。

## 最初の使い方

1. 「プロバイダー」を開き、利用中のサービスを追加します。プリセットを選ぶと一般的な項目が入り、認証情報と必要なエンドポイントだけを補えます。
2. プロバイダーを選び、「適用」を実行します。FyAgent が対象ツールへ書き込む内容を表示してから更新します。
3. 対象のコーディングツールを開き、短いテストリクエストで接続を確認します。
4. 基本の接続が動いてから MCP、Prompts、Skills を追加すると、問題の切り分けが簡単です。

詳しい説明は[日本語マニュアル](docs/user-manual/ja/README.md)にあります。[English](docs/user-manual/en/README.md) と [简体中文](docs/user-manual/zh/README.md) も利用できます。過去の変更は[リリースノート一覧](docs/release-notes/README.md)から確認できます。

## 開発に参加する

このリポジトリでは `mise` を正式な入口として使います。

```bash
mise trust
mise run bootstrap
mise run dev
mise run build
```

Pull Request の前に `mise run check` を実行してください。ツールチェーンと個別チェックは[開発ガイド](docs/fyagent/development/README.md)にあります。大きな変更を始める前に [CONTRIBUTING.md](CONTRIBUTING.md) もご確認ください。

## プロジェクトの経緯とライセンス

FyAgent は CC Switch を基に発展しており、引き継いだコードには元の著作権表示とライセンスを維持しています。FyAgent という製品名、現在の開発、FyAgent 独自の追加部分は FyAgent プロジェクトが管理しています。

FyAgent はソースを利用可能なソフトウェアであり、OSI が定義するオープンソースではありません。FyAgent に帰属するコンポーネントおよび変更部分には [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt) が適用され、商用利用には別途書面による許可が必要です。CC Switch 由来の部分は MIT ライセンスのままです。詳しくは [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md)、[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。
