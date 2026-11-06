<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — 현대적이고 크로스 플랫폼인 Minecraft 런처, 4가지 Catppuccin 테마로 메인 인터페이스 표시">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>작고 빠르고 가벼운 Minecraft 런처</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md"><strong>한국어</strong></a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## 이것이 무엇인가

Conic Launcher는 Rust로 구현되고 UI를 [Slint](https://slint.dev)로 구성한, 작고 빠르고 가벼운 데스크톱
Minecraft 런처입니다. 설치 용량이 작고 실행이 빠르며 리소스 사용량이 적어 탄소 배출 감소에도 도움이 됩니다.

> 이 프로젝트는 현재 초기 Alpha 단계로, 기능과 UI가 빠르게 개선되고 있습니다. 직접 사용해 보시고 문제를 알려 주세요.

## 기능

### 멀티 인스턴스 관리

모드 로더별로 인스턴스를 묶어 보고, 정렬·검색·즐겨찾기를 지원합니다. 인스턴스마다 별도의 설정을 적용할 수 있습니다.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### 모든 주요 로더 지원

Forge, NeoForge, Fabric, Quilt를 완벽하게 지원합니다.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### 모드 및 리소스팩 마켓플레이스

Modrinth와 CurseForge 검색을 내장하여 런처 안에서 프로젝트 상세 정보와 의존성을 확인하고, 클릭 한 번으로 모드나 리소스팩을 인스턴스에 설치하며, 이미 설치된 로컬 모드도 바로 관리할 수 있습니다.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus 크로스 LAN 멀티플레이

추가 도구가 필요 없습니다. 그룹을 만들고 초대 코드를 친구에게 보내면 같은 월드에 함께 입장할 수 있습니다. 런처가 네트워크 환경(NAT 유형)을 감지해 개선 방안을 제안합니다.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### 다양한 로그인 방식

Microsoft 계정 로그인(디바이스 코드 방식), 오프라인 로그인, 그리고 모든 Authlib-Injector /
Yggdrasil 외부 인증 서버를 지원합니다.

<img width="840" height="533" alt="스크린샷 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### 편안한 시작 경험

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### 저장 데이터 및 게임 자료

런처 안에서 바로 세이브의 월드 맵을 둘러보고, 데이터팩과 리소스팩을 관리하며, 게임 스크린샷을 확인할 수 있습니다.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### UI 및 개인 설정

Catppuccin 테마 4종(Mocha · Macchiato · Frappé · Latte)과 고대비 변형을 제공하며, 런처 배경과 배경 음악을 직접 설정할 수 있습니다.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## 다운로드

[GitHub Releases](https://github.com/conic-apps/launcher/releases)
또는 [공식 웹사이트](https://conicmc.app)에서 플랫폼에 맞는 설치 파일을 내려받으세요:

| 플랫폼  | 아키텍처              | 형식                 |
| ------- | --------------------- | -------------------- |
| Windows | x64 · arm64           | MSI · 포터블 exe     |
| macOS   | Apple Silicon · Intel | DMG                  |
| Linux   | x64 · arm64           | deb · rpm · AppImage |

> macOS 11.0(Big Sur) 이상이 필요합니다.

> Windows에서 단일 파일 버전을 사용한다면 시스템에 유효한 `VCRUNTIME140.dll`이 있는지 확인하세요.

## 소스에서 빌드

[Rust 1.88+](https://rustup.rs/)가 설치되어 있어야 합니다. Linux에서는
[빌드 의존성](../../CONTRIBUTING.md#linux-build-dependencies)도 필요합니다.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # 개발 및 디버깅
cargo build --release  # 배포용 빌드
```

## 기여하기

기여를 환영합니다! 시작 방법, 개발 환경 설정, Pull Request 제출 방법은
[CONTRIBUTING.md](../../CONTRIBUTING.md)를 참고하세요.

## 라이선스

이 프로젝트는 [GPL-3.0](../../LICENSE)으로 배포되며, GPLv3 제7조의 모든 추가 조건이 적용됩니다.

## 감사의 글

- [Catppuccin](https://catppuccin.com) — 부드럽고 아름다운 테마 색상
- bangbang93의
  [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) —
  Forge 설치 부트스트랩
- [MCIM](https://mod.mcimirror.top) — CurseForge 중국어 번역 및 미러 캐시
- [HMCL](https://github.com/HMCL-dev/HMCL) — 크래시 처리
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — 로컬 모드 분석 흐름 참고

## 면책 조항

Conic Launcher는 공식 Minecraft 제품이 아니며 Mojang Studios의 승인이나 제휴를 받지 않았습니다.
“Minecraft”는 Mojang AB의 상표입니다. 이 프로젝트의 Minecraft 브랜드 사용은 Mojang Studios의
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">브랜드 및 에셋 가이드라인</a>을 준수합니다.
