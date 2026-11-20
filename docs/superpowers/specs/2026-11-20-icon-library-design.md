# 아이콘 라이브러리 (`web.icon_library`) 설계

작성일: 2026-11-20 · 기준 문서: [docs/plan.md](../../plan.md), [2026-11-10-image-first-addon-design.md](2026-11-10-image-first-addon-design.md)

## 1. 왜

Figma·코드·문서(고해상도 PNG)에 붙일 무료 아이콘을 앱 안에서 찾아 바로 복사한다. 한 번 받아 두면 인터넷 없이 쓴다.
정책의 핵심은 "사용자 데이터가 밖으로 나가지 않는 것"과 "최대한 오프라인"이다. 아이콘 세트와 세트 목록을 **사용자가 누를 때만** 받아 오는 것은 허용한다.

## 2. 목표 / 비목표 / 실패 조건

**목표**

- 웹폰트 탭 맨 아래 "아이콘 라이브러리" — 세트 선택, 스타일 전환, 검색, 격자
- 아이콘을 누르면 오른쪽 패널: 미리보기, 크기·두께·색 조절, 여러 복사 형식
- 추천 3종(Tabler, Heroicons, Remix) + [세트 더 보기]로 무료 라이선스 세트 목록에서 직접 받기
- 세트마다 [받기] · [삭제], 받은 세트 전체에 [업데이트 확인]. 받은 세트는 디스크에 영구 저장

**비목표**

- 앱 시작 시 자동 업데이트 확인·목록 받기 (모두 수동)
- 세트를 앱 설치 파일에 넣기 (추천 3종도 [받기]를 눌러야 함)
- 즐겨찾기, 아이콘 편집, 여러 개 한 번에 내보내기, Iconify 별칭(aliases)·카테고리 탐색

**실패 조건 (동작해도 실패)**

- [받기]·[세트 더 보기]·[업데이트 확인]을 누르지 않았는데 네트워크를 쓴다 (앱 시작 포함)
- 사용자 정보·파일이 요청에 실린다
- 받은 데이터 안의 스크립트·외부 참조가 화면에 들어간다
- 세트 이름(prefix)으로 저장 폴더 밖에 쓴다
- 한 번 받은 뒤 오프라인에서 검색·복사가 안 된다

## 3. 접근 방식

**받는 곳**

| 방식 | 결과 |
| --- | --- |
| **A. npm에서 사용자가 직접 받기** | **선택.** 원본 최신, 우리가 재배포하지 않음 (Remix 라이선스가 2026-01 변경되어 불명확) |
| B. 우리 GitHub Releases에 다시 올리기 | 업데이트가 우리 재업로드에 묶이고 재배포 문제 |
| C. DLL·앱에 넣기 | 세트별 업데이트 불가, 용량, 재배포 문제 |

**패키지 형식**

| 방식 | 결과 |
| --- | --- |
| 원본 패키지 (`@tabler/icons`, `heroicons`, `remixicon`) | 세트마다 폴더 구조가 달라 읽는 코드가 세트 수만큼 |
| **Iconify (`@iconify-json/<prefix>`)** | **선택.** 280개 세트가 같은 JSON 형식, 라이선스 정보 포함, 세트 목록도 npm(`@iconify/collections`)에 있음. 대신 Tabler 검색 태그가 없고 원본보다 갱신이 조금 늦을 수 있음 |

**화면 구조**

| 방식 | 결과 |
| --- | --- |
| 기존 Interactive 구조 + 새 결과 블록 | 고정 오른쪽 옵션 패널(`w-80`)과 "선택에 따라 바뀌는 패널"이 충돌 |
| **전용 화면** | **선택.** 앱 자동 업데이트가 있어 앱 재배포 부담이 작고, 아이콘 데이터는 앱과 무관하게 갱신 |

## 4. 규칙 예외 (CLAUDE.md 함께 수정)

- **도구 전용 UI 금지** → `web.icon_library`는 예외. DLL 도구가 아니라 앱 화면이다.
- **네트워크** → 추가 도구 설치 모듈(`app/src-tauri/src/addons/`)에 아이콘 세트 받기를 넣고, 허용 호스트에 `registry.npmjs.org`를 더한다 (https만). 프론트엔드 http 권한·CSP는 그대로.
- **카테고리** → `web` 탭 이름("웹폰트")은 그대로. 대부분의 세트가 웹폰트로도 제공된다.

## 5. 세트

**추천 3종** (코드 안 목록, 받기 전에도 이름·라이선스를 보여 줌)

| 세트 | prefix | 스타일 칩 | 클래스 복사 |
| --- | --- | --- | --- |
| Tabler (MIT) | `tabler` | 선 · 채움 (`-filled`) — metadata에 없어 목록에서 지정 | `ti ti-<이름>` |
| Heroicons (MIT) | `heroicons` | metadata `suffixes` (Outline 24 · Solid 24 · Solid 20 · Solid 16) | 없음 |
| Remix (Apache-2.0 표기) | `ri` | metadata `suffixes` (Line · Fill) | `ri-<이름>` |

**[세트 더 보기]**

- 누를 때 `@iconify/collections`의 `collections.json`을 받아 목록을 보여 준다 (디스크에 저장하지 않음).
- `hidden`이 아니고 라이선스 spdx가 `MIT`, `Apache-2.0`, `ISC`, `CC0-1.0`인 세트만 (2026-11 기준 193개). 이름·분류·아이콘 수·라이선스 표시, 이름 검색.
- 고른 세트 [받기] → 추천 3종과 같은 방식으로 저장.

**세트 공통 동작**

- 스타일 칩: 추천 목록의 지정 → metadata `suffixes` → 없으면 칩 없음. 아이콘 이름이 `-<suffix>`로 끝나면 그 스타일, 어느 것에도 안 맞으면 `""` 스타일.
- 선 두께: 고른 아이콘 body에 `stroke-width`가 있으면 조절 칸을 보인다.
- 색: 세트 정보에 `palette: true`(컬러 세트)면 색 조절 칸을 숨긴다.
- 크기 기본값: 아이콘 `width`/`height` → 세트 `width`/`height` → 16 (Iconify 기본값).

## 6. 받기 · 업데이트 · 저장

- **prefix 검증:** `^[a-z0-9]+(-[a-z0-9]+)*$`만 받는다. URL과 저장 폴더 이름에 쓰이므로 다른 값은 거부.
- [받기]: `https://registry.npmjs.org/@iconify-json/<prefix>/latest`에서 버전·tarball·`dist.integrity`(sha512) → tarball 받기 → sha512 확인 → `package/icons.json`, `package/info.json`, `package/metadata.json`만 꺼낸다 (나머지 파일·JS는 버린다).
- **정리(sanitize):** `icons.json`의 아이콘 body마다 허용 요소(`g`, `path`, `circle`, `rect`, `line`, `polyline`, `polygon`, `ellipse`, `defs`, `clipPath`, `mask`, `linearGradient`, `radialGradient`, `stop`)와 도형·표현 속성만 남긴다. `script`, `foreignObject`, `image`, `use`, `on*` 속성, `url(#…)` 밖의 외부 참조는 버린다. 정리할 수 없는 아이콘은 뺀다.
- 저장: `%LOCALAPPDATA%\waple-tools\icons\<prefix>\<버전>\`(`app_data_dir()` 아래)에 정리된 `icons.json`·`info.json`·`metadata.json` + `icons\installed.json`(prefix별 버전). 앱을 다시 켜도 남는다.
- [업데이트 확인]: 누를 때만, 받은 세트 전체의 npm 최신 버전과 비교. 새 버전이 있는 세트에 [업데이트] → 새 버전 폴더에 받고 `installed.json`을 바꾼 뒤 예전 폴더 삭제. *(내 결정: 세트 하나씩이 아니라 전체 확인)*
- [삭제]: 세트 폴더와 `installed.json` 항목 삭제. *(내 결정)*
- 배치 작업 실행 중에는 받기를 시작하지 않는다 (기존 설치 모듈과 같게).
- 오류: 네트워크 없음 → "인터넷 연결이 필요합니다", 해시 불일치 → "받은 파일이 손상되었습니다. 다시 받아 주세요.", 잘못된 prefix → "세트 이름이 올바르지 않습니다."

## 7. 화면

```
┌───────────────────────────────────────────────┬─────────────────────┐
│ [Tabler ▾] [선 | 채움] [검색…]  5,880개 · v1.2.46│     (큰 미리보기)    │
│                   [업데이트 확인] [세트 더 보기]│  home               │
├───────────────────────────────────────────────┤  크기   [24] px     │
│  □  □  □  □  □  □  □  □  □  □  □  □           │  두께   ───●── 2    │
│  □  □  □  ■  □  □  □  □  □  □  □  □           │  색     [currentColor]│
│  …                                            │  [SVG 복사]         │
│                                               │  [PNG 복사] 1x 2x 4x │
│                                               │  [클래스 복사] …     │
└───────────────────────────────────────────────┴─────────────────────┘
```

- 세트 선택: 받은 세트 + 아직 안 받은 추천 3종. 안 받은 세트를 고르면 격자 자리에 이름·라이선스·[받기].
- 아이콘을 누르기 전에는 오른쪽 패널 없이 격자가 전체 폭.
- [세트 더 보기]: 대화상자에 목록(이름 검색, 이름·분류·개수·라이선스, [받기]/받음 표시).
- 받은 세트는 세트 선택 옆 메뉴에서 [삭제].
- 검색: 아이콘 이름, 대소문자 무시. 수천 개 격자는 가상 스크롤.
- 오른쪽 패널의 크기·두께·색은 미리보기와 모든 복사 결과에 반영. 색 기본값 `currentColor`.

**복사 형식**

| 버튼 | 결과 |
| --- | --- |
| SVG 복사 | `width`/`height`/`stroke-width`/색 반영한 SVG 코드 (Figma에 붙여 넣기) |
| PNG 복사 | 크기 × 1x·2x·4x, 투명 배경. 웹뷰 canvas로 그려 클립보드에 이미지로 (`currentColor`면 검정) |
| 클래스 복사 | 웹폰트 클래스가 있는 세트만 (`<i class="ti ti-home"></i>`) |
| JSX 복사 | `stroke-width` → `strokeWidth` 등 속성 이름을 바꾼 SVG |
| Data URI 복사 | `url("data:image/svg+xml,…")` |
| SVG 저장 | 저장 대화상자로 파일 저장 |

## 8. 구조 (빌더 참고)

- **src-tauri `addons/`:** 아이콘 세트 모듈 — 받은 세트 목록, 세트 받기, 최신 버전 확인, 삭제, 세트 읽기, 세트 목록(collections) 받기 Tauri 명령. 네트워크는 기존 `net.rs` 클라이언트, 허용 호스트만 추가. tar·gzip·XML 파싱·sha512는 Cargo.lock에 이미 있는 `tar`, `flate2`, `roxmltree`, `sha2`.
- **프론트(FSD):** 웹폰트 탭에서 `web.icon_library`를 고르면 작업대 대신 아이콘 라이브러리 화면. 세트 JSON을 읽어 검색·격자·패널·복사는 프론트에서.
- **카탈로그:** `catalog/tools.json`의 웹폰트 항목 끝. DLL·`builtin()` Tool이 아니므로 사이드바/작업대가 이 id를 전용 화면으로 보내는 방식은 빌더가 정한다.

## 9. 확인 기준

- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`
- `tests/fixtures`의 작은 가짜 Iconify tarball로: 해시 불일치 거부, `script`·`on*`·외부 참조 제거, 잘못된 prefix 거부, 저장 후 다시 읽기, 업데이트 시 예전 버전 삭제
- 프론트 순수 함수(스타일 나누기, SVG 크기·두께·색 적용, JSX 변환, Data URI, 클래스 이름, 라이선스 필터) 단위 테스트
- 한 번 받은 뒤 네트워크를 끊고 검색·모든 복사 동작

## 10. 빌더에게 맡기는 것

- `installed.json`의 정확한 필드
- 화면 세부 수치·문구 (docs/UI-SYSTEM.md 토큰만)
- 가상 스크롤 구현 방법
