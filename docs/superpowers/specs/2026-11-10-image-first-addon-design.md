# LocalTools 이미지 우선 개발 + 추가 설치 도구 설계

작성일: 2026-11-10 · 기준 문서: [docs/plan.md](../../plan.md) (이 문서와 다르면 이 문서가 우선)

승인되면 계획 작성 전에 plan.md(1장 원칙, 2장 구조, 3.7 보안, 8장 단계)와 plan.md 9장의 CLAUDE.md 규칙(네트워크 크레이트 금지 → 설치 모듈만 허용, 도구 = 파일 하나 → 크레이트 하나)을 이 문서에 맞게 고친다.

## 1. 왜 바꾸나

plan.md는 30종 도구를 모두 앱 하나에 넣는 구조다. 이대로면

- 설치 파일이 커진다 (pdfium.dll, 폰트 데이터, SVG 렌더러 등).
- 도구 하나를 고쳐도 앱 전체를 다시 빌드·배포해야 한다.

그래서 **자주 쓰는 기본 도구만 앱에 넣고, 나머지는 앱 안에서 "추가 설치"하는 구조**로 바꾸고, **이미지 기능부터** 개발한다.

## 2. 목표 / 비목표 / 실패 조건

**목표**

- 기본 내장 이미지 도구 7종이 들어간 Windows 앱
- 이미지 나머지 13종은 앱 안에서 추가 설치·제거·업데이트
- 도구 하나 수정 → 그 도구 파일만 빌드해 Releases에 올리면 사용자에게 업데이트됨 (앱 재설치 불필요)
- udrive UI 시스템을 그대로 이식해 데스크톱 프로그램처럼 보이게

**비목표 (이번 범위 밖)**

- PDF·웹·폰트·기타 도구 (plan.md 5·6장). 이미지가 끝난 뒤 같은 추가 설치 구조로 붙인다.
- 앱 자체 자동 업데이트 (Tauri updater). 나중에 udrive 방식으로 붙일 수 있게만 둔다.
- 제3자가 만든 도구. 우리 서명이 있는 도구만 설치된다.

**실패 조건 (동작해도 실패로 본다)**

- 이미지 처리 중에 네트워크를 쓴다.
- 서명이 없거나 틀린 도구 파일이 로드된다.
- 추가 설치 도구가 원본 파일을 직접 옮기거나 덮어쓴다 (반드시 core 배치 엔진을 거쳐야 함).
- 기본 내장 ↔ 추가 설치를 바꾸는 데 도구 코드 수정이 필요하다.

## 3. 바뀌는 원칙 (plan.md 1장)

- **1. 완전 오프라인** → **작업 중 오프라인.** 이미지 처리 중에는 네트워크를 쓰지 않는다. 네트워크는 도구 목록 확인·설치·업데이트에만 쓰고, 텔레메트리와 외부 폰트·스크립트·CDN 로드는 계속 금지.
- 나머지 원칙(AI 미사용, 원본 보존, 확장 구조, 한국어 UI)은 그대로.

## 4. 기본 내장 / 추가 설치 구분

| 구분 | 도구 |
| --- | --- |
| 기본 내장 (2) | `image.optimize`(크기·포맷·품질), `image.edit`(회전·크롭·여백) |
| 추가 설치 (6) | `image.watermark`, `image.svg`, `image.gif_optimize`, `image.sprite`, `image.base64`, `image.icon` |

(2026-11-10 사용성 검토로 처음 19종을 8종으로 합치고, 메타데이터 제거·반응형 이미지 세트는 뺐다. 원본·결과 비교는 도구가 아니라 공통 화면 기능.)

도구 명세(옵션·출력)는 plan.md 4장 그대로. 구분은 나중에 바뀔 수 있다 (6.4 참고).

## 5. 사용자 경험

- 사이드바에는 **모든 도구가 항상 보인다.** 설치 안 된 도구는 이름 옆에 "설치 안 됨" 표시(`text-muted-foreground`).
- 설치 안 된 도구를 누르면 메인 영역에 도구 이름·설명·용량과 **설치** 버튼. 설치 중에는 진행률, 끝나면 바로 일반 도구 화면으로 바뀐다.
- 설치된 추가 도구는 설정(또는 도구 화면)에서 **제거**할 수 있다.
- 새 버전이 있으면 사이드바 항목에 "업데이트" 표시, 누르면 업데이트.
- 인터넷이 없을 때: 설치된 도구는 모두 정상 동작. 설치 버튼만 "인터넷 연결이 필요합니다" 오류.

## 6. 구조

### 6.1 저장소

```
localtools/
├─ crates/
│  ├─ core/            # Tool trait, 배치 엔진, 되돌리기 (plan.md 그대로)
│  ├─ addon-abi/       # 추가 도구 DLL ↔ 앱 사이 C ABI 정의 + export_tool! 매크로 + 앱 쪽 로더
│  ├─ tools/
│  │  ├─ image-optimize/ # 도구 하나 = 크레이트 하나. pub fn tool() -> Box<dyn Tool>
│  │  ├─ image-svg/
│  │  └─ ...
│  └─ cli/
├─ app/
│  ├─ src-tauri/
│  └─ src/             # React UI (FSD: app / pages / widgets / features / entities / shared)
├─ catalog/tools.json  # 전체 도구 목록 (앱에 내장)
└─ docs/
```

plan.md의 "tools 크레이트 하나 + 파일 하나" 대신 **도구마다 크레이트 하나**로 바꾼다. 그래야 도구 하나만 따로 DLL로 빌드할 수 있다.

### 6.2 도구 크레이트 하나가 두 가지로 빌드된다

- **기본 내장:** 앱(`src-tauri`)이 크레이트를 일반 의존성으로 링크하고 `builtin()` 목록에 `image_optimize::tool()`을 넣는다.
- **추가 설치:** 같은 크레이트를 `cdylib`(DLL)로 빌드한다. 크레이트 안에 `addon_abi::export_tool!(tool)` 한 줄이 있고, 이 매크로가 C ABI 함수를 내보낸다. (cdylib 빌드는 cargo feature `addon`일 때만)

### 6.3 C ABI (addon-abi)

Rust에는 안정된 ABI가 없으므로 DLL 경계는 **C 함수 + JSON 문자열**로만 넘긴다.

| 내보내는 함수 | 역할 |
| --- | --- |
| `lt_abi_version() -> u32` | ABI 버전. 앱이 아는 버전과 다르면 로드 거부 |
| `lt_meta() -> *const c_char` | meta + kind + 옵션 스키마 JSON |
| `lt_run(req_json, host) -> *mut c_char` | 실행. 결과(ToolOutput 또는 오류) JSON |
| `lt_free(ptr)` | DLL이 만든 문자열 해제 |

- `host`는 앱이 넘기는 함수 포인터 묶음: 진행률 보고, 취소 확인.
- `req_json`에는 입력 경로, 옵션, **앱이 만든 빈 임시 폴더** 경로가 들어간다. 도구는 그 폴더 안에만 쓰고, 결과로 만든 파일의 상대 경로 목록을 돌려준다. 원본 이동·최종 위치로 옮기기·백업은 앱의 core 배치 엔진이 한다 (plan.md 3.5 그대로). *(내 결정)*
  - 이 계약 하나로 확장자가 옵션에 따라 바뀌는 도구(`convert`, `resize`), 여러 파일을 만드는 Generator·Combine(`favicon`, `sprite` 등)을 모두 처리한다.
  - **기본 내장 도구도 같은 계약**(`Tool::run`이 임시 폴더를 받고 파일 목록을 반환)을 따른다. 그래야 내장↔추가를 바꿀 때 코드 수정이 없다. Phase 0에서 core 시그니처를 이렇게 정한다.
  - Generator·Combine 결과의 최종 위치: 첫 입력 파일이 있는 폴더 아래 `<도구 결과 이름>/` 폴더 (예: `logo-favicon/`). 이름이 겹치면 `(1)`을 붙인다. *(내 결정)*
- **Interactive 결과**는 범용 블록 목록 JSON이다: `text`, `code`(언어 표시), `number`(라벨+값), `color`(HEX 등), `image`(임시 폴더 안 파일 경로), `image_pair`(두 이미지 + 차이 이미지). 앱은 블록 종류별 뷰어만 갖고, 도구별 화면은 만들지 않는다. *(내 결정)*
  - `image_pair` 뷰어가 슬라이더·겹치기·차이 하이라이트 3가지 보기를 제공한다. 비교 로직을 고치면 DLL만, 보기 방식을 고치면 앱을 업데이트한다.
  - `image.crop`의 영역 드래그는 plan.md처럼 나중으로 미룬다. Phase 3의 crop은 비율 프리셋·픽셀 지정·기준 위치 옵션만. *(내 결정)*
- 매크로는 `catch_unwind`로 패닉을 잡아 오류 JSON으로 바꾼다. 패닉이 DLL 경계를 넘지 않게.
- 앱 쪽에서는 로드한 DLL을 `Tool` trait 구현체로 감싸므로, 배치 엔진·UI는 기본 내장과 추가 도구를 구분하지 않는다.

### 6.4 기본 내장 ↔ 추가 설치 바꾸기

도구 코드는 그대로 두고 두 곳만 고친다.

1. `src-tauri`의 `builtin()` 목록과 Cargo 의존성에 넣거나 뺀다.
2. `catalog/tools.json`의 `builtin` 값을 바꾼다.

### 6.5 카탈로그와 원격 목록

- **`catalog/tools.json` (앱에 내장):** 모든 도구의 id, 이름, 카테고리, 설명, `builtin` 여부. 설치 안 된 도구를 사이드바에 보여주는 데 쓴다. 옵션 스키마는 없다 (설치 후 DLL에서 읽음).
- **원격 목록 (`index.json`):** GitHub 공개 저장소 Releases에 올린다. 도구별 다운로드 URL, 크기, sha256, minisign 서명, 버전, ABI 버전.
  - 주소: `https://github.com/cksrlcks/localtools/releases/download/addons/index.json` (소스 저장소 자체를 공개. 고정 태그 `addons` 릴리스에 index.json과 DLL을 올리고 index.json만 갱신). owner는 udrive와 같은 계정으로 가정 *(내 결정)*
- **ABI 버전이 어긋날 때** *(내 결정)*
  - index.json은 도구마다 **ABI 버전별 최신판 하나씩**을 남긴다. 예전 ABI용 DLL은 지우지 않는다. 앱은 자기 ABI와 같은 항목 중 최신을 고른다.
  - 자기 ABI용 항목이 없으면(새 도구가 새 앱만 지원) "앱을 새 버전으로 설치하세요" 표시.
  - 앱을 새로 설치해 ABI가 바뀌면, 설치된 DLL 중 ABI가 안 맞는 것은 로드하지 않고 사이드바에 "업데이트 필요" 표시. 누르면 다시 설치. 자동으로 받지는 않는다.

### 6.6 설치 · 업데이트 · 제거

- 설치 위치: `%LOCALAPPDATA%\LocalTools\addons\<tool-id>\<version>\tool.dll` + `installed.json`(설치된 도구와 버전)
- 설치 순서: 다운로드 → sha256 확인 → minisign 서명 확인(공개키는 앱에 내장, udrive와 같은 방식) → 버전 폴더에 저장 → `installed.json` 갱신 → 로드
- 서명 키는 udrive와 **따로 새로 만든다.** 같은 키를 쓰면 udrive용으로 서명한 파일도 도구로 통과한다. *(내 결정)*
- debug 빌드에서만 개발용 공개키와 로컬 index 경로(환경변수)를 허용한다. release 빌드에는 들어가지 않는다. *(내 결정)*
- 서명·해시가 안 맞으면 파일을 지우고 "설치 파일이 손상되었거나 변조되었습니다" 오류.
- 앱 시작 시에도 설치된 DLL을 로드하기 전에 서명을 다시 확인한다 (설치 후 바꿔치기 방지).
- Windows는 로드된 DLL 파일을 지울 수 없으므로, 업데이트·제거는 새 버전 폴더에 설치 후 `installed.json`만 바꾸고, 예전 폴더는 다음 시작 때 지운다.
- 원격 목록 확인: 앱 시작 시 한 번 백그라운드로 받는다. 실패해도 조용히 넘어가고 "설치" 누를 때 다시 시도.

### 6.7 네트워크와 보안 (plan.md 3.7 변경)

- 네트워크 요청은 **Rust 쪽 설치 모듈 한 곳에서만** 하고, https만 쓴다. 시작 주소는 `github.com`, 리다이렉트는 `*.githubusercontent.com`까지만 따라간다 (GitHub가 다운로드 호스트를 바꿔 온 이력이 있어 이름을 하나로 고정하지 않음). 진짜 보안 경계는 호스트가 아니라 서명 확인이다. *(내 결정)*
- 프론트엔드에는 여전히 http 권한을 주지 않는다. CSP `default-src 'self'` 유지.
- 배치 작업이 실행 중일 때는 다운로드를 시작하지 않는다 (작업 중 오프라인 원칙).

### 6.8 릴리스 (GitHub Actions)

- 태그 `addon/<tool-id>/v<버전>`을 푸시하면 그 도구만 `--features addon`으로 DLL 빌드 → minisign 서명 → `addons` 릴리스에 업로드 → `index.json` 갱신.
- 서명 비밀키는 GitHub Secrets에 둔다.

## 7. UI 시스템 이식 (udrive)

`D:\project\git\udrive\docs\UI-SYSTEM.md` 5장 절차를 따른다. 대상은 Vite + FSD.

| udrive 원본 | localtools 대상 | 바꿀 점 |
| --- | --- | --- |
| `src/index.css` | `app/src/app/styles/index.css` | typography `@plugin`, `.prose-markdown`, `@utility bg-checker` 제거 |
| `src/components/ui/*` | `app/src/shared/ui/*` | `folder-glyph.tsx`, `cloud-mark.tsx` 제외. 내부 import `@/components/ui/*` → `@/shared/ui/*` |
| `src/components/ui/desktop-style.test.ts` | `app/src/shared/ui/` | Vitest 도입 시에만, 경로 수정 |
| `src/lib/utils.ts` | `app/src/shared/lib/utils.ts` | 그대로 |
| `components.json` | `app/components.json` | css 경로, aliases를 `@/shared/ui`, `@/shared/lib/utils`, `@/shared/lib` 로 |
| `docs/UI-SYSTEM.md` | `docs/UI-SYSTEM.md` | 그대로. 이후 UI 작업 기준 문서 |

- 의존성: 문서 5장 목록에서 `@tailwindcss/typography` 제외.
- `vite.config.ts`·`tsconfig.json`에 `@` → `app/src` 별칭, 진입점에서 CSS import.
- `src/features` 화면 코드는 복사하지 않는다. 사이드바·목록 등은 UI-SYSTEM.md 4장 수치를 참고해 새로 만든다.
- 확인: `tsc` + `vite build`, Button · Dialog · ContextMenu를 라이트 / `class="dark"`에서 한 번씩.

## 8. 개발 단계 (plan.md 8장 대체, 이미지 범위)

| Phase | 내용 | 완료 기준 |
| --- | --- | --- |
| 0. 뼈대 | Cargo workspace, Tauri v2 + React + TS, **UI 시스템 이식(7장)**, core의 Tool trait, 더미 도구 | plan.md Phase 0 기준 + 7장 확인 항목 |
| 1. 배치 엔진 | plan.md Phase 1 그대로 | plan.md Phase 1 기준 |
| 2. 기본 UI | plan.md Phase 2 + 사이드바가 `catalog/tools.json`을 읽어 "설치 안 됨" 표시 | plan.md Phase 2 기준 + 설치 안 된 도구 화면 표시 |
| 3. 기본 내장 7종 | 4장 기본 내장 도구 | 도구별 fixtures 테스트, UI에서 폴더 일괄·단일 처리 |
| 4. 추가 설치 구조 | addon-abi, DLL 로더, 설치·업데이트·제거, 서명 검증, 릴리스 워크플로. 첫 추가 도구로 `image.svg` | 로컬 index로 설치→실행→업데이트→제거, 서명 틀린 DLL 거부, 패닉하는 더미 DLL이 앱을 죽이지 않음, `image.svg`를 내장↔추가로 바꿔 빌드해도 둘 다 동작 |
| 5. 추가 도구 나머지 5종 | 4장 추가 설치 도구 | 도구별 테스트, 실제 Releases에서 설치 확인 |
| 6. 배포 | 앱 설치 파일 (NSIS/MSI) | 깨끗한 Windows에서 설치 → 기본 7종은 네트워크 없이 동작, 추가 도구는 설치 후 네트워크 끊고 동작 |

## 9. 빌더에게 맡기는 것

- 크레이트 이름·버전, 다운로드에 쓸 HTTP 크레이트 선택(라이선스·Windows 빌드 확인)
- `index.json`·`installed.json`의 정확한 필드
- 설치 화면 세부 문구와 레이아웃 (UI-SYSTEM.md 토큰만 사용)
- 도구 크레이트 사이 공통 코드(이미지 로드·EXIF 방향 처리 등)를 둘 공용 크레이트 구성
