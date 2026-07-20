# OrcaSVN

English | [简体中文](README.md)

OrcaSVN is a cross-platform SVN desktop client built with Tauri, Rust, and Vue 3. It brings the clarity of a Git-style working-copy experience to SVN while preserving its centralized version-control model.

![OrcaSVN application interface](docs/images/orcasvn-workspace.png)

## Features

- View local changes, unversioned files, conflicts, and missing files in `git status`-style groups
- Checkout, Update, Commit, Add, Delete, Revert, Cleanup, Switch, and Merge
- Browse commit history, inspect file diffs, and view line-by-line blame information
- Available in Simplified Chinese, Traditional Chinese, English, Japanese, and Korean
- Light and dark themes on Windows, macOS, and Linux

> OrcaSVN uses the locally installed `svn` command-line tool; it does not implement the SVN protocol itself.

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

For detailed usage instructions, see [QUICKSTART.md](QUICKSTART.md).

## Local Development

Requirements:

- Node.js 18 or later
- Latest stable Rust toolchain
- SVN CLI 1.10 or later
- Tauri 2 build dependencies for your platform

```bash
npm ci
npm run tauri dev
```

Before submitting changes, run:

```bash
npm run check
```

See [SETUP.md](SETUP.md) for environment setup and troubleshooting, and [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

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
