# OrcaSVN

[简体中文](README.md) | [繁體中文](README.zh-TW.md) | English | [日本語](README.ja.md) | [한국어](README.ko.md)

OrcaSVN is a cross-platform SVN desktop client built with Tauri, Rust, and Vue 3. It brings the clarity of a Git-style working-copy experience to SVN while preserving its centralized version-control model.

![OrcaSVN application interface](docs/images/orcasvn-workspace.png)

Development UI captured before the 0.6.3 release on 2026-10-02 with example workspace data. [Dark theme](docs/images/orcasvn-workspace-dark.png).

Current stable version: **0.6.3**. See the [release notes](docs/releases/v0.6.3.md), [release workflow](docs/releasing.md), and [performance comparison with v0.6.2](docs/workspace-startup-benchmark.md).

## Features

- View local changes, unversioned files, conflicts, and missing files in `git status`-style groups
- Check out, update, select files or directories to commit, add files, revert changes, and clean up working copies
- Save local changes in stashes by file or text hunk and restore them to the original workspace
- Browse commit history, inspect file diffs, and view line-by-line blame information
- Available in Simplified Chinese, Traditional Chinese, English, Japanese, and Korean
- Light and dark themes on Windows, macOS, and Linux

> OrcaSVN uses the locally installed `svn` command-line tool; it does not implement the SVN protocol itself.

Switch, Merge, Resolve, and scheduling deletion of versioned files currently require another SVN client or the command line. Refresh OrcaSVN afterward. The workspace Delete button removes unversioned local files.

## Installation

### Windows

```powershell
winget install OrcaSVN.OrcaSVN
```

You can also download installers for Windows, macOS, and Linux from [GitHub Releases](https://github.com/wustites/OrcaSVN/releases).

After installation, make sure the SVN CLI is available:

```bash
svn --version --quiet
```

## Quick Start

1. Open OrcaSVN and select an existing SVN working copy, or check out a repository.
2. Filter files in the workspace by changed, unversioned, conflicted, or missing status.
3. Select a file to inspect its diff, then review and commit your changes from the Commit page.
4. Run Update before committing and resolve any conflicts first.

For usage instructions, see the [quick start](QUICKSTART.md) and [user guide](docs/user-guide.md) (Simplified Chinese).

## Local Development

Requirements:

- Node.js 18 or later
- Rust **1.88** or newer. Dependencies use Rust edition 2024, which needs Rust 1.85 or newer to compile, and this repository also uses `slice::as_chunks` (stable in 1.88). Rust 1.83 and older cannot pass `cargo check`.
- SVN CLI 1.10 or later, including `svnversion` (used for the highest revision already present in a working copy)
- Tauri 2 build dependencies for your platform

Example Linux (Debian/Ubuntu) packages:

```bash
sudo apt-get install pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf build-essential \
  libssl-dev subversion
```

```bash
npm ci
npm run tauri dev
```

Before submitting changes, run:

```bash
npm run build
npm run test:unit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

`npm run check` runs the frontend type check and build, frontend unit tests, Rust formatting, and `cargo test`.

See [SETUP.md](SETUP.md) for environment setup and troubleshooting, and [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

### Working copy behavior

- On macOS and Linux, SVN child processes use a UTF-8 locale (`en_US.UTF-8` on macOS, `C.UTF-8` on Linux). This keeps Chinese paths, repository URLs, commit messages, and command output intact when a desktop launch inherits a missing, ASCII, or invalid locale. It addresses [#7](https://github.com/wustites/OrcaSVN/issues/7).
- The local revision shown in the app is the highest revision already present in the working copy (the upper bound from `svnversion`). Partial commits still use `--depth empty`, but the revision just committed is not listed as a pending update.
- After add, a valid UTF-8 text file that SVN auto-marked `application/octet-stream` has that property removed, so diff and blame can show the text. Files with NUL bytes or invalid UTF-8 stay binary.
- Checkout success and failure use an explicit state. The UI does not look for the English word `Error` in the output.
- An update that produces text, property, or tree conflicts lists those paths and shows a warning. SVN still exits 0 in that case.
- Diff of a file inside an unversioned directory shows the full content as added. The workspace clears its diff preview when that file is no longer in the status list.

## Project Structure

```text
src/                    Vue 3 frontend
  api/                  Tauri command wrappers
  composables/          Reusable workspace logic
  i18n/                 Localization resources
  stores/               Pinia state
  views/                Application views
src-tauri/src/
  main.rs               Tauri command boundary
  svn/executor.rs       SVN process execution and timeouts
  svn/operations.rs     SVN argument construction
  svn/parser.rs         XML and text result parsing
.github/workflows/      Release workflows
```

## Design Principles

- **Predictable:** UI actions map to explicit SVN commands whenever possible.
- **Review before change:** Status and diffs are shown before committing or reverting changes.
- **Safe argument boundaries:** File targets are separated from command options with `--`.
- **Clear feedback:** Errors retain the original SVN context to make diagnosis easier.

## License

[MIT](LICENSE)
