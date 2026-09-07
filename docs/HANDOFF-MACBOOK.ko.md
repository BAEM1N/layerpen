# 맥북 인계 — LayerPen

## 프로젝트와 배포

- 제품명: LayerPen. 공개 버전: v0.1.0. 이전 내부 버전 0.6.1과 구분합니다.
- 저장소: https://github.com/BAEM1N/layerpen
- 릴리즈: https://github.com/BAEM1N/layerpen/releases/tag/v0.1.0
- Windows x64 설치형 EXE, 포터블 ZIP, 소스 ZIP, 의존성 소스, SHA256 체크섬을 제공합니다.
- `layerpen.app`은 이름을 선택하고 구매 가능 여부만 확인했습니다. 구매·DNS·사이트 배포는 아직 하지 않았습니다.

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
- 신규 기본 캡처 폴더: Pictures/LayerPen. 기존 기본 Pictures/MonitorInk 설정은 새 경로로 전환하되 파일은 이동하지 않습니다. 직접 지정한 다른 폴더는 유지합니다.
- Rust 패키지/내부 실행 파일 `monitor-ink`, 앱 식별자 `dev.personal.monitorink`, 설정 디렉터리 및 `MONITOR_INK_DATA_DIR` 환경변수는 호환성을 위해 유지합니다.
- 포터블은 설치 없이 실행되지만 설정까지 USB 폴더에 보관하는 방식은 아닙니다.
- 확대는 정지 화면이며, GIF는 필기 재생입니다. 라이브 화면 녹화나 공동 편집이 아닙니다.

## 공개 범위와 검증

2026-09-08 v0.1.0 릴리즈 업로드와 36초 소개 영상 렌더링을 완료했습니다. 릴리즈 태그는 `d661f9c54fd054cb17bf90ff487d227f823d18de`입니다. 초기 CI에서 Windows/macOS 테스트·빌드는 통과했고 Linux는 `-lgbm` 링크 실패가 확인되어 main의 CI 설치 목록에 `libgbm-dev`를 추가했습니다. 최신 CI 결과는 Actions에서 확인하세요. 이는 macOS 실제 실행 검증을 대신하지 않습니다.

테스트·빌드 캐시, 개인 화면 캡처, 과거 세션 자료는 Git에 포함하지 않습니다. 배포 산출물은 GitHub Releases로 전달하고 실행 파일을 소스 트리에 넣지 않습니다. `docs/validation/0.1.0.ko.md`에 로컬 검증 범위가 있습니다. CI 성공 여부와 실제 macOS 사용 가능 여부는 별도로 확인하세요.

