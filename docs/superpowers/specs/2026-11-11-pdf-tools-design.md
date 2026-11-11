# PDF 도구 5종 설계

작성일: 2026-11-11 · 기준 문서: [plan.md](../../plan.md) 5장, [이미지 우선 + 추가 설치 설계](2026-11-10-image-first-addon-design.md)

## 1. 왜, 무엇을

웹사이트(ilovepdf 등)에 올려 하던 PDF 작업(이미지로 바꾸기, 이미지 묶기, 나누기, 합치기, 줄이기)을 앱에서 한다. 사용자는 단순한 기능만 쓴다.
이미지 Phase 5(추가 이미지 4종)보다 먼저 한다 (2026-11-11 사용자 결정).

**목표**: plan.md 5장의 `pdf.to_image`, `pdf.from_image`, `pdf.split`, `pdf.merge`, `pdf.compress`를 모두 **추가 설치 도구**로 만든다.

**비목표**: 암호 PDF 열기, 페이지 편집(회전·삭제 UI), 페이지 썸네일 미리보기, OCR·텍스트 추출.

**실패 조건**: 원본 PDF가 바뀌거나 사라진다 · 네트워크를 쓴다 · DLL 말고 다른 파일을 따로 받아야 한다 · 도구 전용 화면 코드가 생긴다.

## 2. 사용자와 정한 것

- 렌더링은 **pdfium 대신 순수 Rust `hayro`**. 추가 DLL이 없어 설치 구조(`tool.dll` 하나)를 그대로 쓴다. 복잡한 PDF가 깨지면 그때 pdfium으로 바꾼다 (도구 크레이트 안에서만 바뀌게 둔다).
- 5종 모두 추가 설치 (`builtin: false`).
- `merge`·`from_image`의 순서는 **기존 입력 목록(필름 띠)에서 끌어서** 바꾼다. 도구별 화면은 만들지 않는다.

## 3. 라이브러리

| 용도 | 크레이트 | 라이선스 |
| --- | --- | --- |
| 페이지 → 비트맵 | `hayro` 0.9 (기본 기능: 표준 14 글꼴·CMap 내장) | Apache-2.0 OR MIT |
| 구조 편집 (나누기·합치기·압축·만들기) | `lopdf` 0.47 (`default-features = false`) | MIT |
| 이미지 디코드·인코드 | 기존 `image`, `waple-tools-image-common`(mozjpeg 등) | 기존 |

plan.md 5장의 "compress 이미지 처리는 pdfium"은 `lopdf` + `image`로 바꾼다 (pdfium 없음).

## 4. 도구별 동작 *(종류·결과 이름은 내 결정)*

| id | 종류 | 결과 |
| --- | --- | --- |
| `pdf.to_image` | Generator | `<이름>/page-001.jpg` … (PDF마다 폴더, plan.md 그대로) |
| `pdf.from_image` | **Combine** | `<첫 이미지 이름>.pdf` |
| `pdf.split` | Generator | `<이름>-split/<이름>_p1-3.pdf` … |
| `pdf.merge` | **Combine** | `<첫 파일 이름>-merged.pdf` |
| `pdf.compress` | PerFile | `<이름>.pdf` (저장 방식 따름, 커지면 원본 유지) |

- `split`을 PerFile이 아닌 Generator로 둔 이유: 기본 저장 방식(원본 자리 + 백업)이면 원본이 `original/`로 옮겨지고 조각 수십 개가 원본 폴더에 흩어진다. 세트 폴더 하나에 모으고 원본은 그대로 둔다.
- 옵션은 plan.md 5장 그대로:
  - to_image: 포맷(JPG/PNG), DPI(72/150/300/직접 입력, 최대 600), JPG 품질, 페이지 범위
  - from_image: 페이지 크기(A4/Letter/이미지 크기), 방향(자동/세로/가로), 여백 mm, 이미지 재압축(JPEG)
  - split: 모드(페이지마다 / 범위 `1-3,5,8-` / N페이지 단위), 범위를 한 파일로 합치기
  - merge: 파일명 북마크 추가
  - compress: 수준(가볍게 / 강하게), 이미지 최대 DPI
- 페이지 범위 문법: `1-3,5,8-` (빈 칸 = 전체). 범위 밖 번호는 오류.
- 암호 PDF: "암호가 걸린 PDF는 처리할 수 없습니다." 손상 PDF: "PDF를 읽을 수 없습니다 (손상되었거나 PDF가 아닙니다)." 배치에서는 그 파일만 실패하고 나머지는 계속된다.
- 글꼴이 들어 있지 않은 한글 PDF: Windows 글꼴(`%WINDIR%\Fonts\malgun.ttf`)을 대체 글꼴로 읽는다 (로컬 파일, 네트워크 아님). *(내 결정)*
- 단순화 *(내 결정)*:
  - from_image: 투명 부분은 흰색으로 채운다. JPEG 원본은 재압축을 끄면 그대로 넣는다.
  - split: 결과에서 원본 북마크(Outlines)는 뺀다 (지운 페이지를 붙잡아 용량이 줄지 않으므로).
  - merge: 원본들의 북마크는 빼고, 옵션을 켜면 파일마다 파일명 북마크 하나를 넣는다.
  - compress 강하게: 8비트 RGB/Gray 이미지(DCT·Flate)만 다룬다. 실제 배치 크기를 계산하지 않고 "이미지가 페이지 전체를 덮는다"고 보고 DPI를 어림한다. 결과가 더 크면 그 이미지는 원래대로 둔다.

## 5. 공통 기능 (앱·core)

### 5.1 Combine 실행 (core 배치 엔진)
- 계획(입력 모으기)은 같다. 순서 = 넣은 순서, 폴더는 이름순.
- 도구를 **한 번** 모든 파일로 실행. 임시 폴더는 첫 파일의 결과 폴더 안.
- 결과는 첫 파일의 결과 폴더에 둔다 (저장 방식이 "다른 폴더"면 그 폴더). 원본은 저장 방식과 상관없이 그대로. 이름이 겹치면 `(1)`.
- 결과 보고: `FileResult` 하나(입력 = 첫 파일, 크기 = 입력 합계). 되돌리기는 기존 기록 방식 그대로.

### 5.2 화면
- 스키마 루트 `x-preview: "none"`: 미리보기 대신 안내 문구. PDF 5종이 쓴다.
- 스키마 루트 `x-page-picker: "<옵션 키>"`: 작업대에 쪽 썸네일 격자를 보여 주고 고른 쪽을 그 옵션(범위 문자열)에 적는다. 썸네일은 앱이 도구를 `{"x-thumbnails": true}` 옵션으로 임시 실행해 받는다 (`thumbs/p-0001.jpg` …). `pdf.to_image`(옵션 `pages`)와 `pdf.split`(옵션 `ranges`)이 쓰며, 둘 다 파일 하나만 받는다(`x-single-input`). 대상 옵션이 비활성이면(분할의 "페이지마다"·"N페이지 단위") 썸네일은 보기만 한다. 썸네일 렌더링은 공용 크레이트 `pdf-render`. 분할의 빈 범위는 "전체를 한 파일로".
- Combine 도구: 작업대(검은 패널)에 입력을 카드 격자로 보여 주고, 카드를 끌어 순서를 바꾼다. 아래 필름 띠는 숨긴다 (2026-11-11 사용자 요청). 포인터 이벤트로 구현 — Tauri 파일 드롭이 켜져 있어 Windows에서 HTML5 드래그가 동작하지 않는다. 키보드는 카드에 초점을 두고 `Alt+←/→`.
- Combine 도구도 Generator처럼 "원본 옆에 저장(접미사)"이 없고 용량 변화를 보이지 않는다. 결과는 모든 입력 카드에 같은 상태로 표시.
- PDF 카드 썸네일은 PDF 아이콘 (썸네일을 만들지 않는다).

## 6. 완료 기준

- 도구별 임시 폴더 테스트 (테스트용 PDF는 `pdf-common`의 `testkit`이 lopdf로 만든다. 바이너리 fixture는 두지 않는다)
- 암호·손상 PDF가 한국어 오류로 실패
- 각 도구를 DLL로 빌드해도 내장과 같은 결과 (`tests/addon.rs`)
- core Combine: 순서 유지, 원본 보존, 되돌리기
- `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, 프론트 `tsc`·`vitest`
