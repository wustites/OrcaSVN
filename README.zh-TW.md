# OrcaSVN

[简体中文](README.md) | 繁體中文 | [English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md)

OrcaSVN 是一款以 Tauri、Rust 和 Vue 3 打造的跨平台 SVN 桌面用戶端。它將 Git 用戶端清晰的工作副本體驗帶到 SVN，同時保留 SVN 的集中式版本控制模式。

![OrcaSVN 軟體介面](docs/images/orcasvn-workspace.png)

0.6.3 發布前的開發版介面，於 2026-10-02 使用範例工作副本資料擷取。[深色主題](docs/images/orcasvn-workspace-dark.png)。

目前正式版本為 **0.6.3**，更新內容見[發布說明](docs/releases/v0.6.3.md)。

## 核心功能

- 以類似 `git status` 的分類檢視本機變更、未納入版本控制的檔案、衝突和遺失檔案
- 簽出、更新、按檔案或目錄選擇提交、加入版本控制、還原與清理工作副本
- 本機貯藏：按檔案或文字區塊儲存修改，並還原至原工作副本
- 瀏覽提交記錄、檢視檔案差異與逐行 Blame 資訊
- 支援簡體中文、繁體中文、英文、日文和韓文
- 支援 Windows、macOS、Linux，以及淺色與深色主題

> OrcaSVN 使用本機安裝的 `svn` 命令列工具，不會自行實作 SVN 通訊協定。

Switch、Merge、Resolve 與受控檔案的刪除排程目前需使用其他 SVN 用戶端或命令列完成，再回到 OrcaSVN 重新整理。工作區的「刪除」按鈕用於刪除未納入版本控制的本機檔案。完整操作說明見[使用者手冊（簡體中文）](docs/user-guide.md)。

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
- Rust **1.88** 或更新版本。相依套件使用 edition 2024（需 1.85 以上），本倉庫另使用 1.88 穩定的 `slice::as_chunks`。1.83 與更早版本無法通過 `cargo check`。
- SVN CLI 1.10 或更新版本（含 `svnversion`，用來讀取工作複本中已有的最高版本）
- 目前平台所需的 Tauri 2 建置相依套件

Linux（Debian/Ubuntu）系統套件範例：

```bash
sudo apt-get install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf build-essential \
  libssl-dev subversion
```

```bash
npm ci
npm run tauri dev
```

提交變更前請執行：

```bash
npm run build
npm run test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`npm run check` 會依序做前端型別檢查與建置、前端單元測試、Rust 格式檢查和 `cargo test`。

環境設定與疑難排解請參閱 [SETUP.md](SETUP.md)，貢獻規範請參閱 [CONTRIBUTING.md](CONTRIBUTING.md)。

### 工作複本行為

- macOS 與 Linux 上，SVN 子程序使用 UTF-8 locale（macOS 為 `en_US.UTF-8`，Linux 為 `C.UTF-8`），避免桌面啟動時繼承缺失、ASCII 或無效 locale 而弄亂中文路徑、倉庫 URL、提交說明與命令輸出。這對應 [#7](https://github.com/wustites/OrcaSVN/issues/7)。
- 介面上的本機版本是工作複本裡已經存在的最高版本（`svnversion` 上界）。依檔案部分提交仍使用 `--depth empty`，但剛提交的版本不會再被更新頁當成尚未套用到本機。
- 加入版本控制時，若 SVN 把有效 UTF-8 文字自動標成 `application/octet-stream`，會移除該屬性，讓差異與 Blame 仍能顯示內容。含 NUL 或非法 UTF-8 的二進位檔維持原樣。
- 檢出結果依成功或失敗狀態顯示，不靠錯誤文字是否包含英文字串 `Error`。
- 更新若產生文字、屬性或樹狀衝突，會列出衝突路徑並顯示警告。SVN 在這種情況下仍以結束碼 0 結束。
- 未追蹤目錄中的檔案也可以查看差異，內容以新增顯示。檔案一旦不再出現在狀態清單中，工作區會清掉對應的差異預覽。

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
