# OrcaSVN

[简体中文](README.md) | [繁體中文](README.zh-TW.md) | [English](README.en.md) | [日本語](README.ja.md) | 한국어

OrcaSVN은 Tauri, Rust, Vue 3로 만든 크로스 플랫폼 SVN 데스크톱 클라이언트입니다. SVN의 중앙 집중식 버전 관리 모델을 유지하면서 Git 클라이언트처럼 명확한 작업 사본 경험을 제공합니다.

![OrcaSVN 애플리케이션 화면](docs/images/orcasvn-workspace.png)

## 주요 기능

- 로컬 변경 사항, 버전 관리되지 않은 파일, 충돌 및 누락된 파일을 `git status`와 유사한 그룹으로 표시
- Checkout, Update, Commit, Add, Delete, Revert, Cleanup, Switch 및 Merge
- 커밋 기록, 파일 Diff 및 줄 단위 Blame 정보 확인
- 중국어 간체, 중국어 번체, 영어, 일본어 및 한국어 지원
- Windows, macOS, Linux와 라이트 및 다크 테마 지원

> OrcaSVN은 로컬에 설치된 `svn` 명령줄 도구를 사용하며 SVN 프로토콜을 직접 구현하지 않습니다.

## 설치

### Windows

```powershell
winget install OrcaSVN.OrcaSVN
```

[GitHub Releases](https://github.com/wustites/OrcaSVN/releases)에서 Windows, macOS 및 Linux용 설치 파일을 다운로드할 수도 있습니다.

설치 후 SVN CLI를 사용할 수 있는지 확인하세요.

```bash
svn --version --quiet
```

## 빠른 시작

1. OrcaSVN을 열고 기존 SVN 작업 사본을 선택하거나 저장소를 Checkout합니다.
2. 작업 공간에서 변경됨, 버전 관리 안 됨, 충돌 또는 누락 상태로 파일을 필터링합니다.
3. 파일을 선택하여 Diff를 확인한 다음 Commit 페이지에서 변경 사항을 검토하고 커밋합니다.
4. 커밋 전에 Update를 실행하고 충돌이 있다면 먼저 해결합니다.

자세한 사용 방법은 [QUICKSTART.md](QUICKSTART.md)를 참고하세요.

## 로컬 개발

요구 사항:

- Node.js 18 이상
- 최신 안정 버전 Rust 도구 체인
- SVN CLI 1.10 이상
- 플랫폼에 필요한 Tauri 2 빌드 종속성

```bash
npm ci
npm run tauri dev
```

변경 사항을 제출하기 전에 다음 명령을 실행하세요.

```bash
npm run check
```

환경 설정과 문제 해결은 [SETUP.md](SETUP.md), 기여 지침은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

## 프로젝트 구조

```text
src/                    Vue 3 프런트엔드
  api/                  Tauri 명령 래퍼
  composables/          재사용 가능한 작업 공간 로직
  i18n/                 현지화 리소스
  stores/               Pinia 상태
  views/                애플리케이션 화면
src-tauri/src/
  main.rs               Tauri 명령 경계
  svn/executor.rs       SVN 프로세스 실행 및 시간 초과
  svn/operations.rs     SVN 인수 구성
  svn/parser.rs         XML 및 텍스트 결과 파싱
.github/workflows/      릴리스 워크플로
```

## 설계 원칙

- **예측 가능성:** UI 작업은 가능한 한 명확한 SVN 명령에 대응합니다.
- **변경 전 검토:** 커밋하거나 되돌리기 전에 상태와 Diff를 먼저 표시합니다.
- **안전한 인수 경계:** `--`를 사용하여 파일 대상과 명령 옵션을 구분합니다.
- **명확한 피드백:** 오류에 원래 SVN 컨텍스트를 유지하여 진단하기 쉽게 합니다.

## 라이선스

[MIT](LICENSE)
