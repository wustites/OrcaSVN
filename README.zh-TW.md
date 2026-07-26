# OrcaSVN

[简体中文](README.md) | 繁體中文 | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

OrcaSVN 是一款以 Tauri、Rust 和 Vue 3 打造的跨平台 SVN 桌面用戶端。它將 Git 用戶端清晰的工作副本體驗帶到 SVN，同時保留 SVN 的集中式版本控制模式。

![OrcaSVN 軟體介面](docs/images/orcasvn-workspace.png)

## 核心功能

- 以類似 `git status` 的分類檢視本機變更、未納入版本控制的檔案、衝突和遺失檔案
- Checkout、Update、Commit、Add、Delete、Revert、Cleanup、Switch 和 Merge
- 瀏覽提交記錄、檢視檔案差異與逐行 Blame 資訊
- 支援簡體中文、繁體中文、英文、日文和韓文
- 支援 Windows、macOS、Linux，以及淺色與深色主題

> OrcaSVN 使用本機安裝的 `svn` 命令列工具，不會自行實作 SVN 通訊協定。

## 安裝

### Windows

```powershell
winget install OrcaSVN.OrcaSVN
```

你也可以從 [GitHub Releases](https://github.com/wustites/OrcaSVN/releases) 下載 Windows、macOS 或 Linux 安裝程式。

安裝後，請確認 SVN CLI 可以使用：

```bash
svn --version --quiet
```

## 快速開始

1. 開啟 OrcaSVN，選擇現有的 SVN 工作副本，或透過 Checkout 簽出儲存庫。
2. 在工作區依變更、未納入版本控制、衝突或遺失狀態篩選檔案。
3. 選擇檔案檢視 Diff，確認後前往 Commit 頁面提交變更。
4. 提交前先執行 Update，並優先解決所有衝突。

更完整的操作說明請參閱 [QUICKSTART.md](QUICKSTART.md)。

## 本機開發

需求：

- Node.js 18 或更新版本
- 最新穩定版 Rust 工具鏈
- SVN CLI 1.10 或更新版本
- 目前平台所需的 Tauri 2 建置相依套件

```bash
npm ci
npm run tauri dev
```

提交變更前請執行：

```bash
npm run check
```

環境設定與疑難排解請參閱 [SETUP.md](SETUP.md)，貢獻規範請參閱 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 專案結構

```text
src/                    Vue 3 前端
  api/                  Tauri 命令呼叫封裝
  composables/          可重複使用的工作區邏輯
  i18n/                 多語系資源
  stores/               Pinia 狀態
  views/                應用程式頁面
src-tauri/src/
  main.rs               Tauri 命令邊界
  svn/executor.rs       SVN 程序執行與逾時處理
  svn/operations.rs     SVN 參數建構
  svn/parser.rs         XML／文字結果解析
.github/workflows/      發佈流程
```

## 設計原則

- **可預期：** 介面操作盡可能對應明確的 SVN 命令。
- **變更前先檢視：** 預設先顯示狀態和差異，再提交或還原變更。
- **安全的參數邊界：** 使用 `--` 將檔案目標與命令選項分隔。
- **清楚的回饋：** 錯誤訊息保留原始 SVN 上下文，便於診斷。

## 授權條款

[MIT](LICENSE)
