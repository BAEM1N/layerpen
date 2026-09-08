현재 상태: Windows 실험 기능으로 구현되었습니다. 이 문서는 초기 제안 기록입니다. 실행 방법과 현재 제한은 [STT-SETUP.md](STT-SETUP.md)를 참고하세요.

# OnPen 로컬 STT 자막 검토

검토일: 2026-09-08. 제안 단계이며 모델 다운로드, 마이크 녹음, 성능 벤치마크 또는 자막 기능 구현은 아직 하지 않았습니다.

## 권장 방향

필기와 함께 쓸 수 있는 선택형 실시간 자막을 추가합니다. 기본 설치 파일에는 모델을 넣지 않고, 사용자가 자막을 처음 켤 때 언어·품질·다운로드 용량을 선택합니다. 자막을 끄면 추론 프로세스와 오디오 입력을 종료하여 메모리를 반환합니다.

첫 비교 후보는 whisper.cpp + 다국어 Whisper base 양자화 모델입니다. tiny는 저사양 후보, small은 정확도 우선 후보로 함께 평가합니다. tiny/base의 한국어·일본어 회의 정확도가 충분하다고 미리 가정하지 않습니다. 영어 전용 `.en` 모델은 글로벌 기본값으로 선택하지 않습니다.

현재 공식 모델 저장소의 Q5_1 파일 크기는 tiny 32.2 MB, base 59.7 MB, small 190 MB입니다. 추론 엔진과 실행 메모리는 포함하지 않은 다운로드 크기입니다.

대안은 sherpa-onnx입니다. streaming Zipformer는 언어별 모델 구성이 필요할 수 있습니다. SenseVoice는 중국어·영어·일본어·한국어·광둥어 후보지만 offline 모델이므로 VAD로 발화를 나누는 준실시간 방식과 진정한 streaming을 구분합니다. 정확도와 지연을 동일한 음성에서 비교한 뒤 기본 엔진을 결정합니다.

whisper.cpp 공식 표의 비양자화 기준 크기/추론 메모리는 tiny 75 MiB/~273 MB, base 142 MiB/~388 MB, small 466 MiB/~852 MB입니다. 양자화 모델의 디스크 크기와 실제 앱 RSS는 별도로 측정해야 합니다. 모델 파일 크기만으로 메모리 예산을 계산하지 않습니다.

## 사용자 경험

- 도구 모음의 CC 버튼 → 입력 소스, 음성 언어, 모델 선택 → 시작.
- UI 언어와 인식할 음성 언어를 별도로 설정합니다. 자동 감지는 선택 기능으로 둡니다.
- 마이크를 먼저 지원하고, 회의 상대방 음성을 위한 시스템 소리 입력을 다음 단계로 추가합니다.
- 화면 아래 이동 가능한 2줄 자막. 미확정 텍스트는 흐리게, 확정 텍스트는 고정하여 잦은 문장 교체를 줄입니다.
- 필기와 별도 창으로 표시하여 지우기·확대·숨기기 동작이 자막을 망가뜨리지 않게 합니다. 기본적으로 클릭을 통과시킵니다.
- 시작/중지와 입력 중 상태를 명확히 표시합니다. 기본은 로컬 추론, 원본 음성·자막 기록 저장 안 함. 사용자가 선택하면 SRT/VTT를 내보냅니다.
- 번역, 화자 구분, 회의 요약, 영상에 자막 입히기는 별도 기능으로 다룹니다. STT만으로 다국어 상호 번역까지 제공하지 않습니다.
- 회의 화면 공유에서 자막이 보이는지 상대방 화면으로 별도 검증합니다.

## 구현 제안

오디오 캡처 → 16 kHz mono 리샘플링 → 제한된 링 버퍼 → VAD → 독립 STT worker → partial/final 이벤트 → Tauri 자막 창.

Rust UI/그리기 경로와 추론 작업을 분리합니다. 초안은 native sidecar 또는 전용 worker로 구성하고, 동일 모델을 세션 중 한 번만 로드합니다. 제한된 큐와 취소 처리를 두어 느린 PC에서 음성이 무한히 쌓이지 않게 합니다. 무음 구간은 VAD와 인식 결과 필터로 억제하고, 겹치는 청크의 중복 문장을 제거합니다.

Windows 시스템 소리는 WASAPI loopback, macOS는 ScreenCaptureKit 경로를 검토합니다. 마이크와 시스템 오디오의 권한·장치 선택은 별도입니다. 동시에 입력받는 경우 중복/에코 문제를 다뤄야 하므로 MVP에서는 한 소스씩 선택합니다. OS 지원 범위를 확인한 뒤 필요한 최소 버전을 문서화합니다.

모델은 버전/체크섬/라이선스를 고정하여 앱 데이터 디렉터리에 보관하고 Git이나 기본 EXE에 넣지 않습니다. 내려받기 취소, 불완전 다운로드 복구, 저장 공간 확인, 삭제 기능을 제공합니다. 추론 엔진과 모델의 배포 조건은 각각 확인합니다.

## 실측 후 결정할 기준

Windows CPU 환경과 Apple Silicon 맥북에서 ko/en/ja/zh 각 언어의 조용한 발화, 잡음, 숫자·고유명사, 코드 용어, 혼합 언어를 비교합니다. 동의받은 샘플 또는 공개 라이선스 음성을 사용합니다.

측정값: 언어에 맞는 CER/WER, 무음 오출력, 첫 자막/확정 자막 지연 p50/p95, real-time factor, RSS, CPU/GPU, 배터리, 필기 입력 지연, 장시간 실행 시 큐 증가. 첫 자막 1~2초는 제품 목표 후보이며 보장된 성능이 아닙니다. 청크 길이·문맥 누적·정확도 사이의 균형을 실제 기기에서 결정합니다.

첫 실험 범위: 마이크 + 다국어 base 양자화 + VAD + 2줄 자막 + 시작/중지. 그다음 small 및 sherpa 후보와 비교하고 시스템 오디오와 SRT/VTT를 추가합니다. v0.1.0 배포 파일은 변경하지 않습니다.

## 공식 참고 자료

- whisper.cpp, 크기·메모리·양자화·플랫폼: https://github.com/ggml-org/whisper.cpp
- 모델 파일: https://huggingface.co/ggerganov/whisper.cpp/tree/main
- 스트리밍 예제: https://github.com/ggml-org/whisper.cpp/tree/master/examples/stream
- sherpa-onnx: https://k2-fsa.github.io/sherpa/onnx/index.html
- streaming 모델: https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html
- Windows loopback: https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording
- macOS ScreenCaptureKit: https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos
