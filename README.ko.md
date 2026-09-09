# Pointory — 오픈소스 화면 필기와 실시간 자막

**화면 위에 쓰고, 텍스트를 더하고, 설명을 이어가세요.**

Pointory(포인토리)는 선생님, 발표자, 개발자를 위한 무료 MIT 라이선스 화면 필기 도구입니다. 선택한 모니터 위에 펜·형광펜·도형·텍스트를 표시하고, 필기를 남긴 채 원래 프로그램을 조작할 수 있습니다. 실시간 자막은 별도 실행 환경이 필요한 실험 기능입니다.

[English](README.md) · **[Windows 다운로드](https://github.com/BAEM1N/pointory/releases/latest)** · [한국어 사용 가이드](Wiki/KR/README.md) · [버그 제보](https://github.com/BAEM1N/pointory/issues/new?template=bug_report.md)

![Pointory의 간결한 툴바, 형광펜, 원과 화살표로 예제 수업 화면을 설명하는 UI](Wiki/assets/pointory-overview.jpg)

*실제 앱 UI와 필기 렌더러를 예제 데이터로 캡처한 브라우저 미리보기입니다. 실제 화면 공유 검증 영상이 아닙니다. 기존 MP4는 재구성 전까지 공개 목록에서 제외했습니다.*

## 다운로드

| 플랫폼 | 파일 | 상태 |
| --- | --- | --- |
| Windows x64 | [설치 EXE](https://github.com/BAEM1N/pointory/releases/download/v0.1.0/Pointory_0.1.0_x64-setup.exe) | v0.1.0 베타 |
| Windows x64 | [포터블 ZIP](https://github.com/BAEM1N/pointory/releases/download/v0.1.0/Pointory-0.1.0-windows-x64-portable.zip) | 압축을 풀고 Pointory.exe 실행 |
| macOS | 공개 DMG 없음 | 이전 M4 빌드 검증, 최신 앱 재실행 대기. [검증 범위](docs/validation/0.2-macos.ko.md) |
| Linux | 배포 바이너리 없음 | X11 개발 대상, Wayland 미지원 |

[체크섬](https://github.com/BAEM1N/pointory/releases/download/v0.1.0/SHA256SUMS.txt) · [릴리스 노트](https://github.com/BAEM1N/pointory/releases/tag/v0.1.0)

WebView2가 필요합니다. 설치 프로그램에서 런타임을 받을 수 있습니다. 현재 설치본은 미서명 베타이며 자동 업데이트는 없습니다. 이전 앱을 종료한 뒤 새 버전을 실행하세요. **일반 필기에는 Python이 필요 없습니다.**

## 1분 시작

1. Pointory를 설치하거나 포터블 ZIP을 풀어 실행합니다.
2. 툴바의 톱니바퀴 **설정** 버튼을 열고 모니터를 선택합니다.
3. 펜으로 그리거나 T 아이콘을 누른 뒤 화면을 클릭해 글자를 입력합니다.
4. Ctrl+Shift+D로 필기와 마우스 조작을 전환합니다.
5. 필요한 내용은 종료 전에 PNG 또는 GIF로 내보냅니다. 필기 기록은 재실행 시 복구되지 않습니다.

## 주요 기능

| 기능 | 활용 | 상세 가이드 |
| --- | --- | --- |
| 펜·형광펜·도형 | 슬라이드와 다이어그램 위에 강조 | [필기](Wiki/KR/03-drawing.md) |
| 텍스트·설정 글꼴·TTF | 시스템 글꼴·TTF 지원, `main`에서 설정 글꼴·크기 조절 추가 | [텍스트와 글꼴](Wiki/KR/04-text-fonts.md) |
| 가로·세로 툴바 | 자주 쓰는 도구는 바로, 도형·색상·더보기는 툴바 옆 패널에서 | [툴바](Wiki/KR/02-toolbar.md) |
| 툴바 옆 설정·5가지 테마 | 작업 흐름과 화면 색상 조절 | [설정](Wiki/KR/02-toolbar.md) |
| 선택·이동·크기·실행 취소 | 완성한 필기 조정 | [편집](Wiki/KR/03-drawing.md) |
| 부분 확대·보드·사라지는 잉크 | 수업과 발표의 강조 | [확대와 보드](Wiki/KR/05-zoom-boards.md) |
| PNG·GIF | 필기 이미지와 재생 저장 | [내보내기](Wiki/KR/06-export.md) |
| 커서 스포트라이트·원형 확대 | 커서 주변만 밝게, Windows 실시간 확대 | [스포트라이트](Wiki/KR/11-spotlight.md) |
| 교실 브라우저 공유 | 같은 네트워크 자료 다운로드·선택적 실시간 화면 | [자료 공유](Wiki/KR/10-classroom-sharing.md) |
| 실험적 실시간 자막 | 입력 장치와 STT 제공자 선택 | [자막](Wiki/KR/07-live-captions.md) |

Epic Pen이나 ZoomIt 같은 화면 필기 도구의 오픈소스 대안을 찾는 분이라면 Pointory의 선택 모니터 오버레이, 텍스트, 툴바 구성을 살펴보세요. 해당 제품과 제휴한 프로젝트는 아닙니다.

## 알아둘 동작

- 화면 좌표 위에 필기하므로 문서를 스크롤하면 글자와 함께 따라가지 않습니다.
- Z 영역 확대는 정지 화면이며, GIF는 필기 재생입니다. 화면 동영상 녹화 기능이 아닙니다.
- 회의에서는 모니터 전체를 공유하고 상대방 화면에서 필기가 보이는지 확인하세요. Zoom·Teams·Meet의 전체 동작은 미검증입니다.
- 필기와 로컬 자막에는 서비스 계정이 필요 없습니다. 앱에는 분석·추적 통합이 없습니다.
- 클라우드 자막은 사용 중 제공자에게 음성을 보내고 비용이 발생할 수 있습니다. API 키는 저장하지 않습니다.
- 설치된 하드웨어·드라이버·모델에 따라 CPU 및 지원 GPU/NPU를 탐색합니다. Apple GPU/Neural Engine 지원은 아직 검증된 기능이 아닙니다.
- 핵심 UI는 한국어·영어·일본어·중국어 간체를 제공합니다. 자막·자료 공유·스포트라이트 설정에는 한·영 병기 항목이 있습니다.

## 교실에서 브라우저로 접속하기

강사 PC 설정의 **자료 공유**에서 파일을 고르고 공유를 시작한 뒤 주소나 QR을 전달합니다. 수강생은 같은 네트워크에서 파일을 내려받습니다. **화면 공유**는 별도로 켜며 선택한 모니터 전체를 최대 1280×720, 약 2 fps로 전송합니다(음성 제외). HTTP 기반 실험적 기능으로 학교 Wi-Fi 격리·방화벽에 따라 접속이 막힐 수 있고 실제 교실 다중 기기 검증은 남아 있습니다. [상세 안내](Wiki/KR/10-classroom-sharing.md).

## 개발과 참여

빌드 명령은 [English README](README.md#build-from-source), 작업 안내는 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요. 재현 가능한 버그, 키보드·접근성 개선, 번역, 다중 모니터 및 Mac 실제 검증을 환영합니다.

v0.2 준비로 M4 Mac의 이전 개발 빌드에서 네이티브 UI·내보내기 시나리오 9개를 검증했습니다. 새 설정 글꼴·크기는 브라우저 검증을 마쳤고 최신 arm64 앱·DMG의 빌드·ad hoc 서명·무결성 검사도 통과했습니다. 최신 앱의 Mac 설치·재실행은 미검증입니다. 전체 화면 캡처, 실제 마이크, Retina·다중 모니터와 Apple GPU/ANE도 미검증이며 Mac 스포트라이트는 확대 없이 밝기 강조만 제공합니다. [Mac 검증 기록](docs/validation/0.2-macos.ko.md) · [후속 준비 사항](docs/HANDOFF-MACBOOK.ko.md).

수업이나 업무에 도움이 되었다면 **Star로 프로젝트를 알려주세요.** 사용 경험과 개선 제안도 환영합니다.

## 라이선스와 이름

프로젝트 코드는 [MIT](LICENSE) 라이선스입니다. 의존성은 [각 라이선스](THIRD-PARTY-NOTICES.txt)를 따릅니다. 제품명은 Pointory(포인토리), 저장소는 `BAEM1N/pointory`입니다. 이전 OnPen·LayerPen 설정과 가져온 글꼴을 이어받으며 내부 앱 식별자는 업그레이드 호환성을 위해 유지합니다. 새 기본 저장 위치는 Pictures/Pointory입니다. 도메인 상태는 [브랜드 안내](docs/BRAND.md)를 참고하세요.
