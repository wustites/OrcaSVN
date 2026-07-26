# OrcaSVN

[简体中文](README.md) | [繁體中文](README.zh-TW.md) | [English](README.en.md) | 日本語 | [한국어](README.ko.md)

OrcaSVN は、Tauri、Rust、Vue 3 で構築されたクロスプラットフォーム対応の SVN デスクトップクライアントです。SVN の集中型バージョン管理モデルを維持しながら、Git クライアントのように分かりやすい作業コピー体験を提供します。

![OrcaSVN アプリケーション画面](docs/images/orcasvn-workspace.png)

## 主な機能

- ローカルの変更、バージョン管理外のファイル、競合、欠落ファイルを `git status` のように分類して表示
- Checkout、Update、Commit、Add、Delete、Revert、Cleanup、Switch、Merge
- コミット履歴、ファイル差分、行単位の Blame 情報を表示
- 簡体字中国語、繁体字中国語、英語、日本語、韓国語に対応
- Windows、macOS、Linux、およびライト／ダークテーマに対応

> OrcaSVN はローカルにインストールされた `svn` コマンドラインツールを使用し、SVN プロトコル自体は実装していません。

## インストール

### Windows

```powershell
winget install OrcaSVN.OrcaSVN
```

[GitHub Releases](https://github.com/wustites/OrcaSVN/releases) から Windows、macOS、Linux 用のインストーラーをダウンロードすることもできます。

インストール後、SVN CLI が利用できることを確認してください。

```bash
svn --version --quiet
```

## クイックスタート

1. OrcaSVN を開き、既存の SVN 作業コピーを選択するか、リポジトリを Checkout します。
2. ワークスペースで、変更済み、バージョン管理外、競合、欠落の各状態からファイルを絞り込みます。
3. ファイルを選択して Diff を確認し、Commit ページから変更をコミットします。
4. コミット前に Update を実行し、競合がある場合は先に解決します。

詳しい操作方法については [QUICKSTART.md](QUICKSTART.md) を参照してください。

## ローカル開発

必要な環境：

- Node.js 18 以降
- 最新の安定版 Rust ツールチェーン
- SVN CLI 1.10 以降
- 各プラットフォーム向けの Tauri 2 ビルド依存関係

```bash
npm ci
npm run tauri dev
```

変更を提出する前に、次のコマンドを実行してください。

```bash
npm run check
```

環境設定とトラブルシューティングについては [SETUP.md](SETUP.md)、コントリビューションのガイドラインについては [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください。

## プロジェクト構成

```text
src/                    Vue 3 フロントエンド
  api/                  Tauri コマンドのラッパー
  composables/          再利用可能なワークスペースロジック
  i18n/                 ローカライズリソース
  stores/               Pinia ステート
  views/                アプリケーション画面
src-tauri/src/
  main.rs               Tauri コマンド境界
  svn/executor.rs       SVN プロセスの実行とタイムアウト
  svn/operations.rs     SVN 引数の構築
  svn/parser.rs         XML／テキスト結果の解析
.github/workflows/      リリースワークフロー
```

## 設計原則

- **予測可能：** UI 操作は、可能な限り明確な SVN コマンドに対応します。
- **変更前に確認：** コミットや取り消しの前に、ステータスと差分を表示します。
- **安全な引数境界：** `--` を使用して、ファイル対象とコマンドオプションを分離します。
- **明確なフィードバック：** 診断しやすいよう、エラーには元の SVN コンテキストを保持します。

## ライセンス

[MIT](LICENSE)
