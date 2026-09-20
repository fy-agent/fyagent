# インストールと更新

[fy-agent/fyagent Releases](https://github.com/fy-agent/fyagent/releases) から OS とアーキテクチャに合うパッケージを取得し、その Release の署名情報、SHA-256、ソース SHA、attestation を確認します。

| OS      | 要件                                 | パッケージ                                                                           |
| ------- | ------------------------------------ | ------------------------------------------------------------------------------------ |
| Windows | Windows 10 以降、x64 / ARM64         | `FyAgent-X.Y.Z-Windows-x64-setup.exe` または `FyAgent-X.Y.Z-Windows-arm64-setup.exe` |
| macOS   | macOS 12 以降、Intel / Apple Silicon | `FyAgent-X.Y.Z-macOS.dmg`                                                            |

Windows ではインストーラーを実行し、ウィザードを完了します。インストールはマシン全体が対象です。信頼に関する警告が出た場合は、配布元とそのビルドの署名状態を確認し、組織のセキュリティ方針に従ってください。

macOS では DMG を開き、`FyAgent.app` を「アプリケーション」にドラッグして起動します。ブロックされた場合は取得元と検証情報を確認し、「システム設定 → プライバシーとセキュリティ」で「このまま開く」の案内に従います。Gatekeeper とダウンロード隔離保護は有効にしておきます。

初回起動時は用途を選ぶか、「跳过引导」（ガイドをスキップ）で一覧を開きます。対象ソフトウェアの検出とインストールは [AI ソフトウェア設定](agents.md) を参照してください。MCP が Node.js / npx や uv / uvx を必要とする場合は、表示される実行環境を先に準備します。

FyAgent の更新は同じ Releases ページから対応パッケージを取得し、各 Release の説明に従います。アンインストールは Windows の「設定 → アプリ」、macOS ではアプリをゴミ箱へ移動します。個人データの既定保存先は `~/.fyagent/` です。ファイルを整理する前に必要なデータをバックアップしてください。

[手冊に戻る](README.md)
