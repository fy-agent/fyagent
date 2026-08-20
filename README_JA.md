<div align="center">
  <img src="assets/brand/github/for-you-gate.svg" width="104" alt="FyAgent For You Gate">
  <h1>FyAgent</h1>
  <p><strong>自分の AI を、自分のものに。</strong></p>
  <p>AI Worker と AI Agent を自分で管理するための、パーソナル・デスクトップコントロールセンター。</p>
  <p><a href="README_EN.md">English</a> · <a href="README.md">简体中文</a></p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/fy-agent/fyagent?style=flat-square&label=release&color=0B66FF"></a>
    <a href="https://github.com/fy-agent/fyagent/actions/workflows/ci.yml"><img alt="CI status" src="https://img.shields.io/github/actions/workflow/status/fy-agent/fyagent/ci.yml?branch=main&style=flat-square&label=CI"></a>
    <img alt="Windows and macOS" src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS-18D3C5?style=flat-square">
    <a href="LICENSING.md"><img alt="Source-available license" src="https://img.shields.io/badge/license-source--available-555B66?style=flat-square"></a>
  </p>
  <p>
    <a href="https://github.com/fy-agent/fyagent/releases/latest"><strong>ダウンロード</strong></a> ·
    <a href="docs/user-manual/ja/README.md">マニュアル</a> ·
    <a href="https://github.com/fy-agent/fyagent/discussions">Discussions</a> ·
    <a href="CONTRIBUTING.md">コントリビュート</a>
  </p>
</div>

## AI 時代のパーソナルコントロールセンター

FyAgent は、AI Agent、AI Worker、AI アシスタントを利用する人のためのアプリです。モデルの入手先、接続できるツール、利用するスキル、従う指示、各種設定といった AI を形づくる選択を、ローカルのデスクトップアプリにまとめます。

Provider、MCP、Prompt といった用語を先に理解する必要はありません。利用者にとっては、AI の知能の供給元、ツールとの接続、仕事の指示に当たります。FyAgent は、分散して見えにくかった選択を、確認・変更しやすい形にします。

現在の FyAgent は、まず具体的な設定管理から始めています。Claude Code、Claude Desktop、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes に対応しています。

WorkBuddy には独立したトップレベルの設定入口があります。上記の対象ツールや Provider ドメインには含まれないため、このツール一覧から WorkBuddy の対応範囲を推測することはできません。

> **リリース状況:** FyAgent は現在も継続的に開発されています。更新前に大切な設定をバックアップし、インストール前に各リリースの信頼情報を確認してください。

## 画面

現在のデスクトップ画面です（簡体字中国語のキャプチャ）。上部バーで Agent ディレクトリ、モデル、Skills、MCP、プロンプト、記憶を切り替えます。

<table>
  <tr>
    <td align="center" width="50%">
      <img src="assets/screenshots/main-zh-1.png" alt="FyAgent のモデル画面：WorkBuddy のサードパーティモデルを管理">
      <br><em>モデル</em>
    </td>
    <td align="center" width="50%">
      <img src="assets/screenshots/main-zh-2.png" alt="FyAgent の Skill マーケット">
      <br><em>Skills</em>
    </td>
  </tr>
  <tr>
    <td align="center" colspan="2">
      <img src="assets/screenshots/main-zh-3.png" alt="FyAgent の MCP 発見画面">
      <br><em>MCP</em>
    </td>
  </tr>
</table>

## ビジョン：AI 時代に持ち歩けるデジタル人格

ここでいう「デジタル人格」は、話し方をまねるアバターではありません。どのモデルを使い、何と接続し、どんなスキルを持ち、どう働き、何を覚えておくのか。人が AI を選び、育て、管理する方法を長く保つための器です。

- **ビジョン:** AI 時代に、誰もが自分のデジタル人格を持ち歩けるようにする。
- **ミッション:** 強力な AI を、管理でき、信頼でき、そばに置ける存在にする。
- **製品の役割:** AI の能力の出どころ、行動、接続先、所有の境界を見える状態に保つ「AI 時代のハンドル」になる。

AI が賢くなるほど、権限を誰に渡したのか、なぜ設定が壊れたのか、ツールを替えるたびになぜ最初からやり直すのかという不安も増えます。FyAgent は、その選択を人の側に残します。全員に同じボットを配るのではなく、一人ひとりが自分の AI を持ち、育て、管理できるようにすることが目標です。

長期記憶とツールをまたいで続くデジタル人格は、今後の製品方向です。次の一覧は、現在のバージョンですでに利用できる機能です。

## 現在できること

| 利用者から見た能力 | 現在の機能                                                                     |
| ------------------ | ------------------------------------------------------------------------------ |
| AI の頭脳          | プロバイダーとモデルを管理し、プリセットまたは互換エンドポイントから切り替える |
| ツール接続         | MCP サーバーを一元管理し、対応する AI ツールへ同期する                         |
| AI スキル          | Skills を管理し、ツールごとの重複設定を減らす                                  |
| 仕事の指示         | Prompts を再利用し、使い慣れた仕事の進め方をツール間で持ち運ぶ                 |
| ルーティングと復旧 | ローカルプロキシ、フェイルオーバールール、モデル疎通確認を利用する             |
| 利用記録           | token 使用量と推定コストをひとつの画面で確認する                               |
| 作業の継続         | セッションやワークスペースを再開し、設定をバックアップ・同期する               |

作業データは既定でローカルの `~/.fyagent` に保存されます。設定更新には SQLite とアトミック書き込みを使い、`fyagent://` からのインポートも書き込み前に変更内容を表示します。

## アーキテクチャ

`React/Vite` レンダラーは Tauri IPC を介して Rust の commands/services を呼び出します。ローカルの Rust 層は SQLite の状態、対象 AI ツールへの設定書き込み、ローカルプロキシを担当します。各層の責任範囲と検証境界は、保守されている[開発ガイド](docs/fyagent/development/README.md)を参照してください。

## クイックスタート

1. [GitHub Releases](https://github.com/fy-agent/fyagent/releases/latest) から、お使いの環境に合うファイルをダウンロードします。
2. 「プロバイダー」を開いて利用中のサービスを追加します。プリセットが一般的な項目を入力します。
3. プロバイダーを選んで「適用」を実行し、FyAgent が書き込む設定を確認します。
4. 対象の AI ツールから短いテストリクエストを送ります。基本接続の確認後に、ツール接続、Skills、仕事の指示を追加します。

詳しい説明は[日本語マニュアル](docs/user-manual/ja/README.md)にあります。[English](docs/user-manual/en/README.md) と [简体中文](docs/user-manual/zh/README.md) も利用できます。

## ダウンロードとリリースの信頼情報

ファイル名は次の形式です。

- macOS: `FyAgent-X.Y.Z-macOS.dmg`、`FyAgent-X.Y.Z-macOS.zip`
- Windows: `FyAgent-X.Y.Z-Windows-x64-setup.exe`、`FyAgent-X.Y.Z-Windows-arm64-setup.exe`

Windows 版は NSIS セットアップのみで、MSI とポータブル ZIP は現在の配布対象ではありません。macOS 版は ad-hoc 署名で、Apple Developer ID では署名されておらず、公証も受けていません。

インストール前にリリースノートを読み、公開されたチェックサム、`signing-status.json`、ビルド証明を確認してください。`NotSigned` は署名状態を示すだけで、安全性の証明ではありません。各 OS の手順は[インストールガイド](docs/user-manual/ja/1-getting-started/1.2-installation.md)、変更履歴は[リリースノート一覧](docs/release-notes/README.md)にあります。

## よくある質問

<details>
<summary><strong>FyAgent のデータはどこに保存されますか？</strong></summary>

既定ではローカル端末の `~/.fyagent` に保存されます。正確な場所とバックアップ方法は[設定ファイル](docs/user-manual/ja/6-faq/6.1-config-files.md)をご覧ください。

</details>

<details>
<summary><strong>インストールや設定について、どこで質問できますか？</strong></summary>

[FAQ マニュアル](docs/user-manual/ja/6-faq/6.2-questions.md)を確認したうえで、FyAgent のバージョン、OS、関連ツール、試した内容を [Q&A](https://github.com/fy-agent/fyagent/discussions/categories/q-a) に投稿してください。再現可能なソフトウェア不具合は [Bug Report](https://github.com/fy-agent/fyagent/issues/new?template=bug_report.yml) を利用してください。

</details>

<details>
<summary><strong>長期記憶や完全なデジタル人格は、すでに利用できますか？</strong></summary>

まだ利用できません。現在のバージョンは、モデル、ツール接続、Skills、仕事の指示、設定、利用記録の一元管理に取り組んでいます。長期記憶とツールをまたいで続く人格は、対応機能が実装・検証されるまでは製品方向として扱います。

</details>

<details>
<summary><strong>FyAgent はオープンソースですか？</strong></summary>

FyAgent はソースを利用可能なソフトウェアであり、OSI が定義するオープンソースではありません。FyAgent 独自のコンポーネントと変更部分には PolyForm Noncommercial License 1.0.0、CC Switch 由来の部分には MIT ライセンスが適用されます。詳しくは[ライセンス説明](LICENSING.md)をご覧ください。

</details>

## コミュニティに参加する

- 使い方とトラブルシューティング：[Q&A](https://github.com/fy-agent/fyagent/discussions/categories/q-a)
- 初期段階の製品アイデア：[Ideas](https://github.com/fy-agent/fyagent/discussions/categories/ideas)
- AI の設定と仕事の進め方を共有：[Show and tell](https://github.com/fy-agent/fyagent/discussions/categories/show-and-tell)
- 再現可能な不具合と具体的な作業：[Issues](https://github.com/fy-agent/fyagent/issues)

コミュニティの行動規範、サポート範囲、貢献方法は [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)、[SUPPORT.md](SUPPORT.md)、[CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## 開発に参加する

初回 checkout には、グローバルにインストールした `mise >= 2026.8.6` が必要です。リポジトリ設定を確認したうえで、対話的な開発は次の順序で開始します。

```bash
mise trust
mise run bootstrap
mise run system:check
mise run dev
```

現在のホスト向けビルドは任意であり、対話的な起動とは別の手順です。

```bash
mise run build
```

検証証拠は範囲ごとに区別します。

- `mise run check` は現在のホストに対する完全なゲートです。ネイティブウィンドウやインストーラーの HIL、署名、公証を証明するものではありません。
- Pull Request の正確な head で成功した `CI / Required` がリモートのマージゲートです。別の SHA や個別ジョブでは代替できません。
- 正式な Release には、正確なソース SHA、前提 CI、annotated tag、正式な Release workflow、公開済みアセットから成る独立した証拠チェーンが必要です。ローカルビルドや Pull Request のチェックでは代替できません。

ツールチェーンと個別チェックは[開発ガイド](docs/fyagent/development/README.md)にあります。

## プロジェクトの経緯とライセンス

FyAgent の前身 VibeKey は、AI の設定と操作を持ち歩ける物理キーボードに収める構想でした。開発を進めるうちに、本当に持ち歩くべきなのはハードウェアではなく、一人ひとりの AI の選択、習慣、仕事の進め方だと分かりました。製品はクロスプラットフォームのデスクトップソフトウェアへ移り、**FyAgent（For You Agent）** になりました。

現在のデスクトップアプリは CC Switch を基に発展しており、継承したコードの著作権表示とライセンスを維持しています。FyAgent の製品名、現在の開発、独自の追加部分は FyAgent プロジェクトが管理しています。

FyAgent 独自のコンポーネントと変更部分には [PolyForm Noncommercial License 1.0.0](LICENSES/PolyForm-Noncommercial-1.0.0.txt) が適用され、商用利用には別途書面による許可が必要です。詳しくは [LICENSE](LICENSE)、[LICENSING.md](LICENSING.md)、[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を参照してください。
