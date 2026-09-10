# Pointory 배포 서명 조사

확인일: 2026-09-10. 현재 미리보기 Windows 설치기는 `NotSigned`, Mac 앱은 ad hoc 서명입니다. Mac의 `codesign verify` 통과는 개발용 무결성 확인이며 Developer ID 서명·Apple 공증을 완료했다는 의미가 아닙니다.

**Windows의 목표는 사용자가 설치·실행할 때 경고를 만나지 않게 하는 것입니다. SmartScreen 경고를 피하는 것이 최우선이면 Microsoft Store 배포를 우선 검토합니다. 웹에서 EXE를 직접 내려받는 경로는 정식 서명 후에도 초기 경고가 남을 수 있습니다.** 직접 EXE 배포에 사용할 서명은 한국 조직 명의라면 Azure Artifact Signing, 개인 오픈소스라면 SignPath Foundation을 검토합니다. Mac은 Developer ID 서명·공증 DMG를 권장합니다. 명의와 서비스 가입 여부는 아직 확정하지 않았습니다.

## EXE 설치·실행 경고에 대한 결론

| 사용자에게 보이는 내용 | 원인과 해결 조건 |
| --- | --- |
| `알 수 없는 게시자` | 신뢰할 수 있는 Authenticode 서명으로 검증된 게시자를 표시합니다. 서명된 파일의 무결성과 인증서 체인도 유효해야 합니다. |
| `Windows의 PC 보호` / 인식할 수 없는 앱 | SmartScreen 평판 문제입니다. Azure·OV·EV 서명 모두 새 앱의 경고를 즉시 없애는 보장이 없으며, EV도 예외가 아닙니다. |
| `이 앱이 디바이스를 변경하도록 허용하시겠어요?` | 관리자 권한 요청인 UAC입니다. 정식 서명을 해도 관리자 권한이 필요하면 동의 창이 뜹니다. |

[Microsoft SmartScreen 설명](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation) · [UAC와 서명된 게시자](https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/how-it-works)

Pointory의 현재 NSIS 설정은 `currentUser`, 생성된 설치 스크립트는 `RequestExecutionLevel user`입니다. Pointory 설치기 자체는 기본적으로 관리자 권한을 요청하지 않습니다. 의존성 설치나 조직 정책을 포함해 어떤 PC에서도 아무 동의 창이 없다는 뜻은 아닙니다.

Microsoft Store에서 설치하는 MSIX는 Microsoft가 서명하고 SmartScreen 다운로드 경고 대상이 아닙니다. Store에 EXE/MSI 방식으로 제출하는 경우도 Store 설치 중 SmartScreen 안내는 없지만, 개발자가 EXE와 내부 PE를 직접 서명해야 하고 UAC는 별개입니다. **Store 등록 후 같은 EXE를 웹에서 직접 내려받는 경로까지 경고가 사라진다고 보장할 수는 없습니다.** [Microsoft 배포 경로별 비교](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)

직접 EXE 배포를 유지한다면 모든 실행 파일·제거기·설치기를 동일한 게시자 명의로 서명하고 타임스탬프를 추가합니다. 최종 GitHub 배포 URL에서 새 Windows 환경의 브라우저로 실제 다운로드해 SmartScreen·게시자·설치·첫 실행을 확인해야 합니다. 로컬 빌드 폴더에서 EXE 실행 성공이나 서명 검사 통과만으로 다운로드 경고가 없다고 판정하지 않습니다.

## 비용과 이용 조건

| 경로 | 공개 비용 | 이용 조건·특징 |
| --- | --- | --- |
| Mac Developer ID + 공증 | Apple Developer Program 연 US$99 | 개인·조직 가입 가능. 실제 한국 청구액은 가입 화면의 현지 통화로 확인. [Apple 한국 가입 안내](https://developer.apple.com/kr/programs/enroll/) |
| Windows Azure Artifact Signing Basic | 월 US$9.99, 월 5,000회, 초과 회당 $0.005 | 조직 신원 검증 후 사용. [Microsoft 요금](https://learn.microsoft.com/en-us/azure/artifact-signing/how-to-change-sku) |
| Windows SignPath Foundation | 승인된 오픈소스 프로젝트 무료 | 재단 명의 인증서. 프로젝트·서명 정책 심사 필요. [공식 조건](https://signpath.org/terms) |
| Windows SSL.com 개인 IV + eSigner | IV 연 $129 + eSigner 연 $180/240회: 합계 연 $309 예시 | 개인 실명 검증과 별도 서명 서비스 비용. 구매 시 선택 옵션을 다시 확인. [IV](https://www.ssl.com/products/software-integrity/code-signing/iv/), [eSigner](https://www.ssl.com/guide/esigner-pricing-for-code-signing/) |
| Microsoft Store MSIX | 신규 개인·회사 계정 등록 무료, MSIX 서명 무료 | Store 패키징과 심사 필요. 현재 NSIS 배포를 그대로 대체하는 것은 아님. [계정 등록](https://learn.microsoft.com/en-us/windows/apps/publish/partner-center/open-a-developer-account), [서명 방식](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) |

USD 공개 가격이며 세금·환율·추가 사용량은 별도입니다. 한국 조직이 Azure Basic을 사용한다면 Apple 멤버십과 합한 기본 비용은 연 **$218.88**입니다. 신청 승인이나 실제 청구액을 보장하는 견적은 아닙니다.

### 한국에서 Azure를 사용할 수 있는가

현행 서비스 Quickstart는 Public Trust 대상에 **한국 조직**을 포함합니다. **개인 개발자는 미국·캐나다만** 지원합니다. Azure 리전의 위치와 신청자의 국가·명의 자격은 서로 다른 조건입니다. 조직용 청구 계정, 법적 사업체 명칭·주소·사업 정보, 사업체 소유 도메인 이메일·웹사이트 등을 맞춰 신원 검증을 신청해야 합니다. 개인사업자의 조직/DBA 인정 여부는 신청 조건에 따라 확인해야 합니다. [현행 Artifact Signing Quickstart](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart)

일부 일반 가이드와 과거 Q&A에는 이전 국가 목록이 남아 있습니다. 실제 신청 때 서비스 포털의 국가·계정 유형과 최신 Quickstart를 함께 확인합니다. 개인 청구 계정으로 조직 검증을 진행할 수 있다고 가정하지 않습니다.

## Mac 직접 배포

1. Apple Developer Program의 개인·조직 명의를 정하고 가입 상태를 확인합니다. 조직 가입에는 법인 자격, D‑U‑N‑S 번호와 계약 권한 등이 필요합니다. 개인/개인사업자는 App Store 판매자가 실명, 조직은 법적 법인명으로 표시됩니다. Pointory라는 제품명만으로 조직 등록을 대신할 수 없습니다. [가입 요건](https://developer.apple.com/help/account/membership/program-enrollment)
2. Account Holder가 Mac에서 생성한 CSR로 **Developer ID Application** 인증서를 발급합니다. 인증서와 대응 개인 키가 함께 있어야 서명할 수 있습니다. 현재 `.app`·`.dmg` 경로에는 `.pkg`용 Developer ID Installer 인증서가 별도로 필요하지 않습니다. [Developer ID 인증서](https://developer.apple.com/help/account/certificates/create-developer-id-certificates/)
3. 내부 실행 코드와 앱을 Hardened Runtime·보안 타임스탬프로 서명하고, DMG를 생성·서명합니다. Apple 공증에 제출해 승인 결과를 확인하고 티켓을 staple합니다. [Apple 공증](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [배포 패키징](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)
4. 최종 파일에서 서명 체인·TeamIdentifier·runtime·timestamp, `stapler validate`, Gatekeeper 평가를 검사하고 다른 Mac에서 브라우저 다운로드 후 첫 실행을 확인합니다. 서명 이후 파일을 바꾸면 다시 서명·공증·해시 생성을 해야 합니다.

Tauri CI에는 인증서 `.p12`와 보호 암호, 서명 식별자를 연결하고, 공증에는 App Store Connect Team API key 또는 키체인에 저장한 앱 전용 암호를 사용할 수 있습니다. Team API key는 한 앱에만 제한되는 키가 아닙니다. 개인 키·인증서 암호·`.p8`는 채팅이나 Git 저장소에 넣지 않고 키체인 또는 보호된 Actions Secrets에서 설정합니다. [Tauri 설정](https://v2.tauri.app/distribute/sign/macos/), [Apple API 키 범위](https://developer.apple.com/documentation/appstoreconnectapi/creating-api-keys-for-app-store-connect-api)

### Pointory에서 먼저 확인할 구현 사항

- 현재 [Tauri 설정](../src-tauri/tauri.conf.json)의 `macOSPrivateApi: true`와 투명 오버레이는 Mac App Store 경로의 제약입니다. Tauri가 설명하는 투명 창 제한과 Apple의 공개 API·sandbox 요건에 맞춘 별도 검토가 필요합니다. 현재 구조 그대로 스토어 승인을 보장할 수 없습니다. [Tauri transparent](https://v2.tauri.app/reference/config/#transparent), [Apple 심사 2.5.1·2.4.5](https://developer.apple.com/app-store/review/guidelines/)
- 마이크 설명은 [Info.plist](../src-tauri/Info.plist)에 있지만 별도 오디오 입력 entitlement는 아직 설정하지 않았습니다. Hardened Runtime 배포 전에 `com.apple.security.device.audio-input`과 Python STT 자식 프로세스의 권한 흐름을 검토하고 최종 서명 앱으로 실제 녹음을 확인해야 합니다. 현재 마이크 목록 조회 성공만으로 녹음 준비 완료를 판정하지 않습니다. [Apple Audio Input entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.audio-input)

## Windows 직접 배포

앱 실행 파일과 내부 PE를 먼저 서명하고, NSIS 빌드 중 제거기를 서명한 뒤 완성된 설치기에 서명합니다. 마지막에 ZIP과 SHA256 목록을 생성합니다. SHA256 파일 서명과 RFC 3161 SHA256 타임스탬프를 사용하고 `signtool verify /pa /all /v`, `Get-AuthenticodeSignature`로 확인합니다. [Microsoft SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool), [타임스탬프](https://learn.microsoft.com/en-us/windows/win32/seccrypto/time-stamping-authenticode-signatures)

현재 Tauri Windows 설정에는 서명 서비스가 연결되어 있지 않습니다. 명의·서비스를 정한 뒤 `bundle.windows.signCommand`와 릴리스 CI를 연결하면 됩니다. [Tauri 사용자 지정 서명](https://tauri.app/distribute/sign/windows/#custom-sign-command)

**새 앱은 서명해도 SmartScreen 경고가 바로 사라지지 않을 수 있습니다.** Microsoft는 EV 인증서에도 더 이상 즉시 평판을 부여하지 않는다고 설명합니다. 비싼 EV 인증서를 경고 제거 수단으로 선택하기보다 동일한 게시자 명의로 릴리스를 유지해야 합니다. 서명은 출처·변조 확인이고 파일 평판은 별도입니다. [Microsoft SmartScreen](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation)

한국 개인 명의의 직접 배포 대안인 SSL.com은 IV 검증 문서에서 한국 신분증·여권 등의 지원을 안내합니다. 인증서 가격에 토큰이나 클라우드 서명 비용이 포함되는지 확인해야 합니다. [신원 확인 안내](https://www.ssl.com/guide/identity-validation-for-ssl-com-certificates-a-complete-guide/)

### 오픈소스 SignPath 후보

Pointory의 MIT 라이선스·공개 소스·문서·릴리스는 신청 기반이 됩니다. 다만 프로젝트 유지보수와 공개 이력, 소스와 빌드의 연결, MFA·서명 정책, 릴리스 승인 절차를 심사하므로 승인 여부는 미정입니다. 검증된 게시자는 SignPath Foundation이며 자체 회사 명의 인증서와 다릅니다. GitHub 연동에서는 서명 요청까지 연결되는 작업이 GitHub-hosted runner에서 실행되도록 구성해야 합니다. 로컬에서 만든 EXE를 임의로 보내 서명하는 절차가 아닙니다. [재단 조건](https://signpath.org/terms), [GitHub 빌드 연동](https://docs.signpath.io/trusted-build-systems/github)

## 스토어 배포는 별도 경로

Microsoft Store의 **MSIX 제출**은 Microsoft가 서명합니다. **EXE/MSI 등록**은 개발자가 설치기와 내부 PE를 직접 서명해야 하며, 버전별 HTTPS URL·무인 설치·설치 중 다운로드 없는 설치기가 요구됩니다. 현재 Pointory는 NSIS와 WebView2 `downloadBootstrapper` 설정이므로 스토어 전용 패키징이 필요합니다. [EXE/MSI 요구 사항](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msi/app-package-requirements)

Windows의 경고 없는 설치 목표에는 **배포 명의 결정 → Store용 패키징·기능 검증 → 심사 → 새 PC에서 실제 Store 설치 확인**을 우선합니다. 직접 EXE 배포를 병행한다면 별도로 서명·타임스탬프·웹 다운로드 검증을 진행하고 초기 SmartScreen 경고 가능성을 남겨 둡니다. Mac은 Developer ID 서명·공증과 새 Mac에서 다운로드·실행 확인을 진행합니다. 현재 현장 미리보기는 정식 신뢰 배포물로 표시하지 않습니다.
