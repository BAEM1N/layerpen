# Pointory(포인토리) v0.2 Mac 검증과 인계

**2026-09-11 업데이트:** 최신 아이콘·모델 준비 코드를 반영한 Mac 앱·DMG를 빌드했습니다. 실제 음성 엔진 설치, Whisper tiny 다운로드, 오프라인 CPU 파일 인식과 Python 3.9 환경 복구를 확인했습니다. Mac이 로그인 화면 상태라 최신 GUI·실제 마이크 확인은 현장에 남아 있습니다. [최신 검증 기록](validation/0.2-macos-models.ko.md).

배포 명의·비용·직접 배포와 스토어 차이는 [서명 조사](SIGNING.ko.md)를 참고하세요.

현재 제품명은 **Pointory(포인토리)**이며 저장소는 [BAEM1N/pointory](https://github.com/BAEM1N/pointory)입니다. 처음 받을 때는 `git clone https://github.com/BAEM1N/pointory.git` 후 `cd pointory`로 이동합니다. Rust 패키지/바이너리는 `pointory`, 내부 앱 식별자는 업그레이드 호환을 위해 `dev.personal.monitorink`를 유지합니다. 기본 저장 폴더는 Pictures/Pointory이며 `POINTORY_DATA_DIR`를 우선 사용합니다. 이전 환경변수와 설정은 호환됩니다.

`pointory.app`은 우선 도메인 후보이며 아직 구매·DNS 연결·사이트 배포를 하지 않았습니다.

2026-09-10에 M4 Mac mini(macOS 26.2, 16 GB, 단일 1920×1080·배율 1)에서 재부팅 전 네이티브 WKWebView 시나리오 9개를 검증했습니다. 툴바·설정·텍스트·투명 PNG·GIF·밝기 스포트라이트·로컬 공유 제어와 메뉴 막대 아래로 밀리던 창 위치 수정이 포함됩니다. 최신 원격 자동 검사는 검증 전용 항목을 포함한 Rust 41개·JavaScript 29개·STT 18개·현장 실행기 4개, 합계 92개 통과입니다. Windows 브라우저에서 Mac의 예제 자료를 실제 다운로드하는 별도 LAN 검사도 통과했습니다. [구체적인 방법과 결과](validation/0.2-macos.ko.md)를 먼저 확인하세요.

새 설정 글꼴·크기는 브라우저 40조합(4언어 × 5크기 × 2너비)과 한국어·영어 FontFace 로드를 검증했습니다. 최신 메인 스레드 잠금 수정까지 포함한 일반 arm64 `.app`·DMG 빌드, 체크섬·읽기 전용 마운트·내용, ad hoc 리소스 서명 검사가 모두 통과했고 Windows NSIS 최종 빌드도 완료했습니다. 재부팅 후 로그인된 데스크톱 세션이 없어 새 Mac 설정 시나리오와 최신 앱의 실제 설치·재실행은 미검증입니다.

화면 기록 권한은 아직 미승인이며 실제 마이크 입력 장치가 없어 전체 화면 캡처·마이크 음성 인식 검증은 남아 있습니다. 별도로 합성 음성 파일을 일반 앱의 실제 Whisper `tiny`·CPU 경로로 인식하고 모델 캐시를 이용한 오프라인 재실행까지 확인했습니다. 현재 Mac 시스템 오디오 경로는 지원하지 않고 Apple GPU/ANE도 미구현입니다. 개발용 ad hoc 서명·DMG 빌드가 Developer ID 서명·공증·스토어 배포 완료를 뜻하지 않습니다.

토요일 현장에서는 [Mac 현장 확인표](MAC-FIELD-CHECK.ko.md)에 따라 바탕화면의 `Pointory-Check` 준비 폴더로 자동 검사와 실제 마우스·한글 키보드 입력을 확인합니다. 기본 확인과 화면 기록·마이크 권한이 필요한 선택 검사를 구분하며, 준비 폴더 실행과 DMG 설치·실행 결과도 각각 기록합니다.

## SSH로 준비할 것
- Mac 주소(IP 또는 VPN 호스트), 사용자명, 포트(기본 22), 작업할 폴더.
- 시스템 설정 → 일반 → 공유 → 원격 로그인에서 작업 계정만 허용.
- 작업용 SSH 공개키를 Mac 계정의 authorized_keys에 등록. 개인키/비밀번호를 채팅에 붙이지 않습니다.
- 접속 가능한 네트워크, 전원 연결, 로그인된 데스크톱 세션.

SSH로 소스 수정·빌드·자동 테스트는 가능하지만 GUI 실행 확인에는 실제 로그인 세션과 사용자 승인, 필요시 원격 화면 공유가 필요합니다. Mac에서 실행하거나 화면을 직접 확인하며 진행하는 방식도 가능합니다. 화면 및 시스템 오디오 기록, 마이크, 필요시 손쉬운 사용/입력 모니터링 권한은 해당 기능에서 요청할 때 사용자가 승인합니다. 전체 디스크 접근은 기본 요구사항이 아닙니다.

Xcode Command Line Tools, Rust, Node, Python 환경은 접속 후 확인하고 준비합니다. Apple ID/개발자 인증서는 로컬 기능 검증을 시작하는 데 필요하지 않습니다. 서명·공증·스토어 제출은 별도 배포 단계입니다.

## v0.2 남은 실제 검증
- 로그인된 데스크톱에서 최신 앱 재실행, 설정 글꼴·12~20px 크기 변경과 저장 확인.
- Retina/다중 모니터/음수 좌표, 툴바 이동과 설정 도킹, 화면 캡처 권한.
- 한글 IME·글꼴·텍스트 크기, 전역 단축키·클릭 통과, 전체화면 앱과 Spaces.
- 마이크 선택·중지·다시 시작, 시스템 오디오 지원 경로.
- Apple Silicon용 Metal/Core ML/ANE 실행 경로를 선택한 모델로 구현·측정. 현재 CTranslate2 CPU / Intel OpenVINO 경로가 Apple GPU/ANE 지원을 뜻하지 않습니다.
- sandbox 및 공개 API 오버레이: 현재 macOSPrivateApi는 App Store 준비의 차단 항목입니다.

참고: https://support.apple.com/guide/mac-help/allow-a-remote-computer-to-access-your-mac-mchlp1066/mac
참고: https://v2.tauri.app/start/prerequisites/

---
아래는 OnPen 시기의 이전 인수인계 기록입니다. 당시 제품명·배포 구성·영상·도메인·검증 내용을 보존한 것이며, 현재 사용법이나 배포 상태로 해석하지 마세요. 최신 상태는 위 내용과 README를 우선합니다.

# 맥북 인계 — OnPen

## 프로젝트와 배포

- 제품명: OnPen. 공개 버전: v0.1.0. 이전 내부 버전 0.6.1과 구분합니다.
- 저장소: https://github.com/BAEM1N/layerpen
- 릴리즈: https://github.com/BAEM1N/layerpen/releases/tag/v0.1.0
- Windows x64 설치형 EXE, 포터블 ZIP, 소스 ZIP, 의존성 소스, SHA256 체크섬을 제공합니다.
- `onpen.app`은 이름을 선택하고 구매 가능 여부만 확인했습니다. 구매·DNS·사이트 배포는 아직 하지 않았습니다.

## 맥북에서 시작

```sh
git clone https://github.com/BAEM1N/layerpen.git
cd layerpen
npm ci
npm test
```

앱 개발에는 Rust stable, Xcode Command Line Tools 및 Tauri macOS 사전 요구사항이 필요합니다. Node.js 22 이상을 권장합니다.

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --features custom-protocol
npm run dev
```

Windows 빌드는 검증했지만 macOS 실행·패키징은 아직 검증하지 않았습니다. 현재 번들 대상은 NSIS입니다. macOS 출시 전에 app/dmg 대상을 검토하고 화면 기록·접근성 권한, 투명 오버레이, 전체화면/Spaces, 모니터 좌표와 Retina 배율, 전역 단축키, PNG/GIF 내보내기를 실제 기기에서 확인해야 합니다. 서명·공증은 아직 없습니다.

## 제품 소개 영상

`video/`에 Remotion 프로젝트와 lockfile, 실제 UI 이미지가 있습니다. 자세한 명령은 `video/README.md`를 참고하세요.

```sh
cd video
npm ci
npm run studio
# 최종 영상 생성
npm run render
```

36초 / 1080p / 30fps. 영어 문구 중심, 무음. `src/index.jsx`에서 장면을 수정합니다. 설정 화면은 실제 UI이고 그리기·확대 화면은 설명용 애니메이션입니다. 다음 편집 단계에서 실제 사용 녹화와 한국어·일본어·중국어 영상 버전을 추가할 수 있습니다.

## 유지해야 할 동작

추가 검토: [로컬 STT 실시간 자막 제안](LIVE-CAPTIONS-PROPOSAL.ko.md). 아직 구현하거나 모델을 실행하지 않았으며, whisper.cpp 다국어 base 양자화와 sherpa-onnx 후보를 실제 Windows/Mac 음성에서 비교하는 단계부터 시작합니다.

- UI: 한국어·영어·일본어·중국어 간체, 시스템 언어 자동 선택과 영어 fallback.
- 신규 기본 캡처 폴더: Pictures/OnPen. 기존 기본 Pictures/MonitorInk 설정은 새 경로로 전환하되 파일은 이동하지 않습니다. 직접 지정한 다른 폴더는 유지합니다.
- Rust 패키지/내부 실행 파일 `onpen`, 앱 식별자 `dev.personal.monitorink`, 설정 디렉터리 및 `ONPEN_DATA_DIR` 환경변수는 호환성을 위해 유지합니다.
- 포터블은 설치 없이 실행되지만 설정까지 USB 폴더에 보관하는 방식은 아닙니다.
- 확대는 정지 화면이며, GIF는 필기 재생입니다. 라이브 화면 녹화나 공동 편집이 아닙니다.

## 공개 범위와 검증

2026-09-08 v0.1.0 릴리즈 업로드와 36초 소개 영상 렌더링을 완료했습니다. 릴리즈 태그는 `d661f9c54fd054cb17bf90ff487d227f823d18de`입니다. 초기 CI에서 Windows/macOS 테스트·빌드는 통과했고 Linux는 `-lgbm` 링크 실패가 확인되어 main의 CI 설치 목록에 `libgbm-dev`를 추가했습니다. 최신 CI 결과는 Actions에서 확인하세요. 이는 macOS 실제 실행 검증을 대신하지 않습니다.

테스트·빌드 캐시, 개인 화면 캡처, 과거 세션 자료는 Git에 포함하지 않습니다. 배포 산출물은 GitHub Releases로 전달하고 실행 파일을 소스 트리에 넣지 않습니다. `docs/validation/0.1.0.ko.md`에 로컬 검증 범위가 있습니다. CI 성공 여부와 실제 macOS 사용 가능 여부는 별도로 확인하세요.
