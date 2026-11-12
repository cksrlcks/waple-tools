# waple tools 개발 작업지시서

웹사이트에 업로드해서 하던 이미지·PDF·웹 작업을 내 PC에서 처리하는 **Windows 데스크톱 앱**을 Rust로 만든다.
이 문서는 Claude Code가 단계별로 작업할 때 기준으로 삼는 문서다.

---

## 1. 개요와 원칙

- **대상 사용자:** 디자이너, 웹퍼블리셔, 일반 사무 업무
- **대상 OS:** Windows 10/11 (x64). macOS는 고려하지 않는다.
- **기능 수:** 이미지 10종, PDF 5종, 영상 5종, 웹·폰트 1종, 기타 5종. 이후 계속 추가된다.
- **개발 순서:** 이미지 기능 먼저. 자주 쓰는 도구만 앱에 내장하고 나머지는 앱 안에서 추가 설치한다.
  상세 설계: [specs/2026-11-10-image-first-addon-design.md](superpowers/specs/2026-11-10-image-first-addon-design.md) — 이 문서와 다르면 설계 문서가 우선.

반드시 지킬 원칙:

1. **작업 중 오프라인:** 파일 처리 중에는 네트워크를 쓰지 않는다. 네트워크는 추가 도구 목록 확인·설치·업데이트와 앱 자동 업데이트에만 쓴다. 텔레메트리, 외부 폰트·스크립트·CDN 로드는 금지.
2. **AI 최소 사용:** AI/머신러닝 모델은 사용자가 지시한 업스케일(Real-ESRGAN)·배경 제거(BiRefNet)에만 쓴다 (2026-11-12). 모델은 도구 DLL에 넣어 오프라인으로 돌리고, 목록에 AI 뱃지를 단다.
3. **원본 보존:** 원본 파일을 삭제하거나 덮어쓰지 않는다. 원본은 `original/` 폴더로 옮기거나 그대로 둔다.
4. **확장 구조:** 도구 1개 추가 = Rust 파일 1개 + 레지스트리 등록 1줄. 도구별 UI 코드는 만들지 않는다.
5. **한국어 UI:** 화면 문구와 오류 메시지는 한국어로 작성한다.

---

## 2. 기술 스택과 저장소 구조

### 기술 스택

| 영역 | 선택 |
|---|---|
| 앱 프레임워크 | Tauri v2 |
| 프론트엔드 | React + TypeScript + Vite |
| 백엔드 | Rust (stable 최신), Cargo workspace |
| 설치 파일 | Tauri 번들러 (NSIS 또는 MSI) |

### 주요 크레이트 후보

구현 시 최신 버전·라이선스·Windows 빌드 가능 여부를 확인하고, 더 나은 대안이 있으면 제안 후 교체한다.

| 용도 | 크레이트 |
|---|---|
| 이미지 디코딩/인코딩 공통 | `image` |
| 고속 리사이즈 | `fast_image_resize` |
| WebP | `webp` (libwebp) |
| JPEG 고압축 | `mozjpeg` |
| PNG 최적화 | `oxipng` |
| EXIF 읽기/제거 | `kamadak-exif`, `img-parts` |
| SVG 렌더링 | `resvg` / `usvg` |
| SVG 최적화 | `oxvg` 검토, 없으면 usvg 기반 정리 |
| GIF | `gif`, `color_quant` |
| 아이콘 파일 | `ico`, `icns` |
| PDF 렌더링(→이미지) | `hayro` (순수 Rust, 2026-11-11 pdfium-render에서 변경 — 추가 DLL 없음) |
| PDF 구조 편집 | `lopdf` |
| 영상 디코딩/인코딩 | `windows` (Media Foundation, 2026-11-11 — OS에 든 H.264·AAC 코덱, 코덱 DLL 번들 없음) |
| AI 추론 (업스케일·배경 제거) | `ort` (onnxruntime CPU, 도구 DLL에 정적 링크, 2026-11-12) |
| 폰트 서브셋 / WOFF2 | `subsetter` 또는 `allsorts`, WOFF2 인코더 크레이트 검토 |
| 색상 팔레트 | `kmeans_colors` 또는 median-cut 직접 구현 |
| 텍스트 비교 | `similar` |
| JSON | `serde_json` (`preserve_order`) |
| 폴더 순회 / 병렬 | `walkdir`, `rayon` |
| 옵션 스키마 | `serde`, `schemars` |
| 에러 | `thiserror`, `anyhow` |
| DLL 로드 | `libloading` |
| 서명 확인 | `minisign-verify` |
| 해시 | `sha2` |
| 다운로드 | 설치 모듈 전용 HTTP 크레이트 (`ureq` 등, 구현 시 선택) |

### 저장소 구조

```
waple-tools/
├─ Cargo.toml              # workspace
├─ CLAUDE.md               # Claude Code 작업 규칙 (9장)
├─ catalog/tools.json      # 전체 도구 목록(이름·설명·카테고리·builtin 여부). 앱에 내장
├─ crates/
│  ├─ core/                # Tool trait, 레지스트리 타입, Job, 배치 엔진, 백업/되돌리기 (GUI 의존성 없음)
│  ├─ addon-abi/           # 추가 도구 DLL ↔ 앱 C ABI, export_tool! 매크로, 앱 쪽 로더
│  ├─ tools/               # 도구 구현. 도구 하나당 크레이트 하나
│  │  ├─ image-webp/       # pub fn tool() -> Box<dyn Tool>, feature "addon"이면 DLL로 빌드
│  │  ├─ image-svg-to-png/
│  │  └─ ...
│  └─ cli/                 # core + tools를 쓰는 CLI (테스트·자동화용)
├─ app/
│  ├─ src-tauri/           # Tauri 명령, builtin() 목록, 추가 도구 설치 모듈, 번들 설정
│  └─ src/                 # React UI (FSD, UI 시스템은 docs/UI-SYSTEM.md)
├─ docs/
└─ tests/fixtures/         # 테스트용 샘플 이미지/PDF/폰트
```

---

## 3. 공통 설계

### 3.1 Tool trait (`crates/core`)

```rust
pub trait Tool: Send + Sync {
    fn meta(&self) -> ToolMeta;                    // id, 이름, 카테고리, 설명, 입력 확장자
    fn kind(&self) -> ToolKind;                    // 아래 4가지 중 하나
    fn options_schema(&self) -> serde_json::Value; // schemars로 생성한 JSON Schema
    fn run(
        &self,
        ctx: &JobContext,                          // 진행률 보고, 취소 확인
        input: &[PathBuf],
        out_dir: &Path,                            // 앱이 만든 빈 임시 폴더. 도구는 여기에만 쓴다
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError>;            // 만든 파일의 상대 경로 목록 또는 Interactive 결과 블록
}
```

- 기본 내장 도구와 추가 설치 도구가 같은 계약을 따른다. 원본 이동·최종 위치로 옮기기·백업은 배치 엔진이 한다.
- Interactive 결과는 범용 블록(`text`, `code`, `number`, `color`, `image`, `image_pair`) 목록이다. 앱은 블록 종류별 뷰어만 갖는다.

- `ToolMeta.id` 형식: `카테고리.이름` (예: `image.optimize`, `pdf.split`)
- 옵션 구조체는 `#[derive(Deserialize, JsonSchema)]`로 정의한다. 기본값, 최소/최대, 설명(한국어)을 스키마에 담는다.
- 프론트엔드는 스키마를 읽어 옵션 폼을 자동 생성한다. 슬라이더, 숫자, 선택, 체크박스, 텍스트, 색상, 파일 선택 정도를 지원하면 된다.

### 3.2 도구 종류 (`ToolKind`)

| 종류 | 설명 | 예 |
|---|---|---|
| `PerFile` | 파일마다 독립 처리. 단일 파일과 폴더 일괄 처리 모두 지원 | WebP 변환, 압축, 리사이즈 |
| `Combine` | 여러 파일 → 결과 1개. 순서 지정 필요 | PDF 병합, 이미지 → PDF, 스프라이트 |
| `Generator` | 파일 1개 → 결과 세트(폴더) | 파비콘, 앱 아이콘, 반응형 세트 |
| `Interactive` | 파일을 만들지 않고 화면에 결과 표시 (복사/저장 버튼 제공) | JSON 포맷터, 텍스트 비교, 팔레트 추출, Base64 |

### 3.3 레지스트리

- 기본 내장 도구: `app/src-tauri`의 `builtin()` 목록에 등록한다.
- 추가 설치 도구: 설치된 DLL을 로드해 `Tool`로 감싼다. 설치·업데이트·서명 확인은 설계 문서 6.5~6.6.
- `catalog/tools.json`에 모든 도구가 있고, `list_tools`는 카탈로그 + 설치 상태(내장/설치됨/설치 안 됨/업데이트 있음/업데이트 필요) + 설치된 도구의 스키마를 넘긴다.
- 내장 ↔ 추가 전환은 `builtin()` 목록·Cargo 의존성과 카탈로그의 `builtin` 값만 바꾼다.

### 3.4 출력 정책

`PerFile` 도구는 아래 중 하나를 선택한다. 기본값은 `InPlaceWithBackup`.

1. **`InPlaceWithBackup`** (기본): 결과물을 원본 위치에 저장하고, 원본은 `original/`로 이동.
2. **`KeepOriginal`**: 원본은 그대로 두고 결과물을 `이름_suffix.ext`로 같은 폴더에 저장.
3. **`OutputFolder`**: 원본은 그대로 두고 지정 폴더에 결과물 저장 (하위 폴더 구조 유지).

단일 파일 처리에도 같은 정책을 적용한다. 단일 파일의 `original/`은 그 파일이 있는 폴더 바로 아래에 만든다.
`Combine`, `Generator`는 원본을 건드리지 않고 새 파일/폴더만 만든다.

### 3.5 배치 엔진 (`crates/core/batch.rs`)

입력: 파일 목록, 폴더, 또는 둘의 혼합. 폴더는 "하위 폴더 포함" 옵션을 둔다.

1. 순회 시 `original/` 폴더, 숨김 폴더, 도구가 지원하지 않는 확장자는 제외한다.
2. 결과물은 같은 폴더에 임시 이름(`.waple-tools-tmp-<uuid>`)으로 먼저 만든다.
3. 성공했을 때만 원본을 `<작업 루트>/original/<상대 경로>`로 이동하고, 임시 파일을 최종 이름으로 바꾼다.
4. `original/`에 같은 이름이 있으면 `name (1).ext`처럼 번호를 붙인다. 어떤 경우에도 덮어쓰지 않는다.
5. "결과가 원본보다 크면 원본 유지" 옵션(기본 켜짐). 이 경우 임시 파일을 지우고 결과 리포트에 "건너뜀(용량 증가)"으로 남긴다.
6. 확장자가 바뀌는 변환(예: PNG → WebP)은 결과물이 새 확장자로 저장되고 원본만 `original/`로 간다.
7. 모든 생성/이동 내역을 `<작업 루트>/original/.waple-tools-log/<job_id>.json`에 기록한다.
8. **되돌리기:** 로그를 역순으로 적용해 결과물을 삭제하고 원본을 원래 위치로 되돌린다. 되돌리는 도중 사용자가 파일을 수정했으면(해시 불일치) 그 파일은 건너뛰고 알린다.
9. **dry-run:** 실제 변경 없이 처리 대상, 출력 경로, 건너뛸 파일 목록을 반환한다. UI 미리보기에 쓴다.
10. 파일 단위 실패는 전체를 멈추지 않는다. 결과 리포트에 파일별 성공/실패/건너뜀과 사유를 담는다.
11. `rayon`으로 병렬 처리한다. 동시 작업 수는 설정에서 바꿀 수 있다(기본: CPU 코어 수 - 1).
12. 취소하면 진행 중인 파일까지만 처리하고, 그 시점까지의 로그로 되돌리기가 가능해야 한다.

### 3.6 Tauri 명령

| 명령 | 설명 |
|---|---|
| `list_tools` | 도구 목록, meta, 옵션 스키마 |
| `preview_job` | dry-run 결과 |
| `run_job` | 실행. 진행률은 이벤트로 전송 |
| `cancel_job` | 취소 |
| `undo_job` | 로그 기반 되돌리기 |
| `run_interactive` | Interactive 도구 실행, 결과 반환 |
| `get_settings` / `set_settings` | 설정 (로컬 JSON 파일) |
| `install_tool` / `uninstall_tool` | 추가 도구 설치·업데이트·제거. 진행률은 이벤트로 전송 |

### 3.7 보안 설정

- Tauri capability에 http, shell(임의 실행), 원격 URL 권한을 넣지 않는다. 프론트엔드는 네트워크를 쓰지 않는다.
- CSP는 `default-src 'self'`로 외부 리소스를 차단한다. 폰트·아이콘도 앱에 내장한다.
- 외부 프로그램은 실행하지 않는다. 필요한 라이브러리(pdfium 등)는 DLL로 번들한다.
- 네트워크는 Rust 설치 모듈과 앱 업데이터(tauri-plugin-updater)에서만, https로 `github.com` 시작 → `*.githubusercontent.com` 리다이렉트까지만. 배치 작업 중에는 다운로드를 시작하지 않는다.
- 앱 자동 업데이트: 켤 때 `releases/latest/download/latest.json`을 확인하고, 사이드바 버튼으로 설치 → 다시 시작. 업데이트 파일은 minisign 서명(공개키는 `tauri.conf.json`, 비밀키는 저장소 밖과 GitHub Secrets)이 맞아야 설치한다. 릴리스는 `v*` 태그 push → `.github/workflows/release.yml`.
- 추가 도구 DLL은 sha256 + minisign 서명(앱 내장 공개키, udrive와 다른 키)을 설치 때와 로드 때 모두 확인한다. 맞지 않으면 로드하지 않는다.

---

## 4. 기능 명세: 이미지 (10종)

공통: 입력 JPG, PNG, WebP, GIF, BMP, TIFF 중 도구가 지원하는 것. EXIF 방향은 자동 반영한다. 메타데이터 유지/제거 옵션을 공통으로 둔다(기본: 제거).
파일 도구 화면은 공통으로 입력 썸네일과 **원본 ↔ 결과 비교**(슬라이더·나란히 보기, 용량 변화)를 실행 전에 보여 준다 — 도구별 화면이 아니라 모든 이미지 도구가 같이 쓰는 기능.

| # | ID | 기능 | 구분 | 종류 | 주요 옵션 | 출력 |
|---|---|---|---|---|---|---|
| 1 | `image.optimize` | 이미지 최적화 | 기본 내장 | PerFile | **크기**: 원본 유지 / 가로·세로 기준 / 정확한 크기 / SNS 규격 프리셋(OG 1200×630, X 1600×900, 인스타 1080×1080·1080×1350·1080×1920), 맞춤(비율 유지·늘리기·크롭 채우기·여백 추가), 확대 금지(기본 켜짐) → **포맷**: 원본 유지·JPG·PNG·WebP, 투명 부분 배경색(JPG) → **품질**: 품질 지정 / 목표 용량(KB, 품질 이진 탐색), 무손실(WebP), PNG 색상 수 → 메타데이터 | 처리된 이미지. JPG=mozjpeg, PNG=oxipng(+선택적 색상 수 줄이기), WebP=libwebp |
| 2 | `image.edit` | 자르기 · 회전 | 기본 내장 | PerFile | 묶음 스위치 2개(처음엔 모두 꺼짐): 회전·뒤집기(90°/180°/270°·좌우·상하, JPG는 가능하면 무손실) → 자르기(직접 지정: 미리보기에서 영역 드래그, 비율 고정 자유·1:1·4:3·16:9 등 / 픽셀: 크기와 기준 위치). 여백 기능은 쓰임이 적어 뺐다 | 처리된 이미지. 한 번에 파일 하나만 받는다(`x-single-input`). 영역 드래그는 스키마 `x-region`으로 선언, 화면은 Ark UI ImageCropper |
| 3 | `image.watermark` | 워터마크 | 추가 설치 | PerFile | 종류(텍스트/로고 이미지), 텍스트·폰트(Windows 시스템 글꼴)·크기(짧은 변 %)·색, 로고 파일(옵션 폼 파일 선택 `format: "file"`)·크기 %, 위치(9방향/타일 -30°), 불투명도, 여백 | 워터마크 적용 이미지 (입력과 같은 포맷, 16비트 유지) |
| 4 | `image.svg` | SVG 도구 | 추가 설치 | PerFile | 모드: 최적화(소수점 자릿수, 주석·메타데이터 제거, 빈 그룹 제거, viewBox 유지) / PNG로 내보내기(배율 또는 px, 배경 투명/단색) | `.svg` 또는 `.png`. 최적화는 roxmltree로 SVGO 기본 정리만(oxvg는 MPL 의존성·0.0.x라 쓰지 않음), PNG는 resvg. 외부 파일·URL 리소스는 읽지 않음. PNG 모드만 `x-allow-larger-when` |
| 5 | `image.gif_optimize` | GIF 최적화 | 추가 설치 | PerFile | 색상 수, 프레임 건너뛰기(N개마다 1개), 리사이즈 % | 최적화된 `.gif` |
| 6 | `image.sprite` | 스프라이트 생성 | 추가 설치 | Combine | 배치(가로/세로/격자), 간격 px, 클래스 접두사, 2x 레티나 | `sprite/` 폴더에 `sprite.png` + `sprite.css` (+ `sprite@2x.png`). 폴더로 묶어 이름이 겹쳐도 CSS의 url이 맞는다 |
| 7 | `image.base64` | 이미지 → Base64 | 추가 설치 | Interactive | 출력 형식(Data URI / CSS background / `<img>` 태그) | 화면 표시 + 복사 버튼. 큰 파일은 경고 |
| 8 | `image.icon` | 아이콘 생성 | 추가 설치 | Generator | 대상: 웹 파비콘 / 앱(Android·iOS·Windows·macOS 다중 선택), 배경색, 여백 %, 둥근 모서리, 적응형 아이콘 전경 이미지(선택) | 아래 세트 |
| 9 | `image.upscale` | 업스케일 (AI) | 추가 설치 | PerFile | 배율 2배/4배 | 입력과 같은 포맷(알파 있으면 PNG). Real-ESRGAN x4plus(BSD-3) 128px 타일 추론, 알파는 일반 리사이즈. 결과 긴 변 16384px·1억 화소 상한, 미리보기 없음 |
| 10 | `image.bg_remove` | 배경 제거 (AI) | 추가 설치 | PerFile | 배경: 투명 / 단색(색 선택) / 마스크만 | `이름.png`. BiRefNet_lite(MIT) 1024px 마스크를 원본 크기로 올려 알파로 적용 |

AI 도구 공통(2026-11-12): onnxruntime(`ort`)를 정적 링크하고 모델을 `include_bytes!`로 넣어 도구 DLL 하나로 설치한다. 실행 중 네트워크 없음. 모델은 git에 넣지 않고 `scripts/fetch-models.sh`가 빌드 전에 받는다(sha256 고정). 세션은 프로세스당 하나, 추론은 한 번에 하나. 목록에 AI 뱃지(카탈로그 `"ai": true`).

웹 파비콘 세트: `favicon.ico`(16/32/48), `favicon-16x16.png`, `favicon-32x32.png`, `apple-touch-icon.png`(180), `android-chrome-192x192.png`, `android-chrome-512x512.png`, `site.webmanifest`, HTML `<link>` 코드

앱 아이콘 세트:

- **Android:** `mipmap-mdpi`(48) ~ `mipmap-xxxhdpi`(192)의 `ic_launcher.png`, `ic_launcher_round.png`, 적응형 아이콘 `ic_launcher_foreground.png`(108dp 기준 각 밀도), `ic_launcher_background` 색상, `mipmap-anydpi-v26/ic_launcher.xml`, Play 스토어용 `512×512`
- **iOS:** `AppIcon.appiconset/` 폴더(필요 사이즈 전체) + `Contents.json`, App Store용 `1024×1024`(투명 없음)
- **Windows:** `.ico` (16/24/32/48/64/128/256)
- **macOS:** `.icns`

---

## 5. 기능 명세: PDF (5종)

| # | ID | 기능 | 종류 | 주요 옵션 | 출력 |
|---|---|---|---|---|---|
| 1 | `pdf.to_image` | PDF → 이미지 | Generator | 포맷(JPG/PNG), DPI(72/150/300 또는 직접 입력), JPG 품질, 페이지 범위 | `이름/page-001.jpg` ... (PDF마다 폴더) |
| 2 | `pdf.from_image` | 이미지 → PDF | Combine | 페이지 크기(A4/Letter/이미지 크기), 방향(자동/세로/가로), 여백 mm, 맞춤(비율 유지), 이미지 재압축 여부 | `.pdf` 1개 |
| 3 | `pdf.split` | 분할 / 페이지 추출 | Generator | 모드(페이지마다 / 범위 지정 `1-3,5,8-` / N페이지 단위), 범위를 하나의 파일로 합치기 | `이름-split/이름_p1-3.pdf` 등 |
| 4 | `pdf.merge` | 병합 | Combine | 파일 순서(UI 드래그), 북마크에 파일명 추가 | `.pdf` 1개 |
| 5 | `pdf.compress` | 압축 | PerFile | 수준(가볍게: 객체 정리·스트림 재압축 / 강하게: 내부 이미지 다운샘플·JPEG 재압축), 이미지 최대 DPI | 압축된 `.pdf`. 텍스트 선택 가능 상태 유지 |

공통: 암호화된 PDF와 손상된 PDF는 명확한 한국어 오류 메시지로 건너뛴다. 렌더링은 hayro, 구조 편집·압축은 lopdf + image를 쓴다. 세부 결정은 [PDF 도구 설계](superpowers/specs/2026-11-11-pdf-tools-design.md).

---

## 6. 기능 명세: 영상, 웹·폰트, 기타

### 영상 (5종, 2026-11-12)

| # | ID | 기능 | 종류 | 주요 옵션 | 출력 |
|---|---|---|---|---|---|
| 1 | `video.compress` | 영상 용량 줄이기 | PerFile | 해상도(원본/1080p/720p/480p, 짧은 변 기준·확대 안 함), 화질(높음/보통/낮음 = 화소당 비트 수, 또는 목표 용량 MB — 넘으면 비트레이트를 낮춰 최대 3번 다시), 소리 유지 | `이름.mp4` (H.264 High + AAC 128kbps) |
| 2 | `video.convert` | 영상 포맷 변환 | PerFile | 출력: MP4(H.264+AAC) / WMV(WMV9+WMA) / M4A(소리만) / MP3(소리만), 소리 유지(MP4·WMV) | 해상도 원본, 화질은 용량 줄이기의 '높음' |
| 3 | `video.to_gif` | 영상 → GIF | PerFile | 시작(초), 길이 0.5~30초, 초당 10/15/20프레임, 너비 원본/640/480/320(확대 안 함), 반복 | `이름.gif`. 프레임 수 × 화소 1억 상한 |
| 4 | `video.from_gif` | GIF → 영상 | PerFile | 투명 부분 배경(흰색/검정) | `이름.mp4`. 프레임 지연 그대로, 홀수 크기는 1px 덧대기, 4096px 이하 |
| 5 | `video.thumbnail` | 영상 썸네일 추출 | Generator | 방식(균등 N장 / N초 간격 / 지정 시간), JPG/PNG, 너비 원본/1280/640/320, 최대 100장 | `이름/이름_00m12s.jpg` |

공통: 추가 설치 도구, 미리보기 없음(`x-preview: "none"`). 인코딩·디코딩은 Windows Media Foundation(외부 프로그램 실행이나 코덱 번들 없이 OS 기능을 부른다, 그래픽카드 인코더가 있으면 씀). 읽을 수 있는 형식은 Windows 코덱이 정한다 — HEVC는 'HEVC 비디오 확장', Windows N은 '미디어 기능 팩'이 필요하다는 한국어 오류로 안내한다.

### 웹·폰트 (1종)

| # | ID | 기능 | 종류 | 주요 옵션 | 출력 |
|---|---|---|---|---|---|
| 1 | `web.font_subset` | 웹폰트 서브셋 / WOFF2 변환 | Generator (2026-11-11, PerFile에서 변경: 저장 방식의 접미사·`(1)`이 CSS url을 깨뜨림) | 글자 범위(KS X 1001 완성형 2,350자 / 영문·숫자·기호 / 직접 입력 / 텍스트 파일에서 추출 / 조합), 출력(WOFF2, WOFF, TTF 다중 선택) | `<이름>-subset/` 폴더에 서브셋 폰트 + `@font-face` CSS. 처리 전후 용량 표시(`x-report-size`). TTF는 skeravar(hb-subset 포팅, GSUB·GPOS 유지), OTF(CFF)는 allsorts(커닝·합자 빠짐) |

### 기타 (5종, 2026-11-11 색상 팔레트를 웹·폰트에서 옮김, QR 코드 생성 추가)

| # | ID | 기능 | 종류 | 주요 옵션 | 출력 |
|---|---|---|---|---|---|
| 1 | `misc.palette` | 색상 팔레트 추출 | Interactive | 방식(주요 색상 = MMCQ / 역할별 = node-vibrant 6역할 Vibrant·Muted·Dark·Light + 본문 글자색), 색상 수(3~12), 정렬(비율순/밝기순) | 색상 칩, HEX·RGB·HSL, 비율 %. 복사 버튼, CSS 변수/JSON으로 내보내기 |
| 2 | `misc.rename` | 파일명 일괄 변경 | PerFile (특수) | 규칙 조합: 찾아 바꾸기(정규식 지원), 접두사/접미사, 번호 붙이기(시작값·자릿수), 대소문자 변경, 공백→하이픈, 날짜 추가 | 이름 변경. **반드시 변경 전/후 미리보기 표 제공**, 충돌 시 실행 불가 표시, 되돌리기 지원. `original/` 백업 대신 로그 기반 되돌리기만 사용 |
| 3 | `misc.json` | JSON 포맷터 | Interactive | 들여쓰기(2/4/탭), 압축(minify), 키 정렬 | 정렬된 JSON, 오류 시 줄·열 위치 표시. 파일 열기/저장 지원 |
| 4 | `misc.diff` | 텍스트 비교 | Interactive | 표시(좌우 / 통합), 공백 무시, 대소문자 무시 | 줄 단위 + 줄 내부 글자 단위 하이라이트. 파일 2개 드롭 또는 붙여넣기 |
| 5 | `misc.qr` | QR 코드 생성 | Interactive (입력 파일 없음, `x-input: "none"`) | URL, 크기(px), 여백(칸), 오류 복원(L/M/Q/H), 전경색·배경색 | PNG 미리보기(이미지 복사·저장) + SVG 코드(복사·저장). 저장은 덮어쓰지 않고 `이름 (1).png` |

---

## 7. UI 구성

- **왼쪽 사이드바:** 카테고리(이미지 / PDF / 웹·폰트 / 기타)별 도구 목록, 상단 검색, 즐겨찾기 고정
- **메인 영역 (파일 도구):**
  1. 드롭 영역: 파일 또는 폴더 드래그 앤 드롭, "파일 선택" / "폴더 선택" 버튼
  2. 입력 목록: 파일명, 크기, 상태. Combine 도구는 드래그로 순서 변경
  3. 옵션 폼: 스키마 기반 자동 생성. 도구별 마지막 사용 옵션 기억
  4. 출력 정책 선택(3.4)과 하위 폴더 포함 체크
  5. 미리보기 버튼: dry-run 결과(대상 수, 건너뜀 수, 출력 경로)
  6. 실행 → 진행률 바, 파일별 상태, 취소 버튼
  7. 결과 리포트: 성공/실패/건너뜀, 전체 용량 변화(예: 45.2MB → 12.1MB, -73%), "폴더 열기", "되돌리기"
- **메인 영역 (Interactive 도구):** 좌측 입력 / 우측 결과 2단 구성, 복사·저장 버튼
- **설정:** 동시 작업 수, 기본 출력 정책, 기본 메타데이터 처리, 작업 로그 보관 기간, 테마(시스템/라이트/다크)
- 다크 모드 지원, 최소 창 크기 1024×700

---

## 8. 개발 단계와 완료 기준

각 단계는 별도 브랜치/세션으로 진행하고, 완료 기준을 모두 만족해야 다음 단계로 넘어간다.

### Phase 0. 뼈대
- Cargo workspace, Tauri v2 앱, React+TS 템플릿 구성
- udrive UI 시스템 이식 (설계 문서 7장, 기준 문서 `docs/UI-SYSTEM.md`)
- `core`: Tool trait(3.1, `out_dir` 계약), ToolMeta, ToolKind, JobContext(진행률·취소), 에러 타입
- 더미 도구 1개(파일 복사)로 UI → Rust → 결과 흐름 연결
- **완료 기준:** `cargo test` 통과, 앱 실행 → 도구 목록 표시 → 더미 도구 실행 → 진행률 표시, `tsc`·`vite build` 통과, Button·Dialog·ContextMenu 라이트/다크 확인

### Phase 1. 배치 엔진
- 3.4 출력 정책, 3.5 배치 엔진 1~12 전체, 되돌리기, dry-run
- CLI에서 `waple-tools run <tool-id> <경로> --opts '{...}'`, `waple-tools undo <job_id>` 지원
- **완료 기준:** 임시 폴더 기반 테스트 통과 — 처리/백업, 이름 충돌, 용량 증가 시 원본 유지, 중간 실패 시 원본 보존, 취소 후 되돌리기, 되돌리기 후 원상 복구 확인

### Phase 2. 기본 UI
- 7장의 파일 도구 화면 전체, 스키마 기반 옵션 폼, 설정 화면
- 사이드바는 `catalog/tools.json`을 읽어 모든 도구를 표시하고, 설치 안 된 도구는 "설치 안 됨" + 설명 화면(설치 버튼은 Phase 4에서 동작)
- Interactive 결과 블록 뷰어(`text`, `code`, `number`, `color`, `image`)
- **완료 기준:** 더미 도구로 폴더 드롭 → 미리보기 → 실행 → 결과 → 되돌리기까지 UI에서 동작, 설치 안 된 도구 화면 표시

### Phase 3. 기본 내장 이미지 2종
- `image.optimize`(크기·포맷·품질), `image.edit`(회전·자르기), 공통 원본·결과 비교 미리보기
- (처음엔 도구 7종으로 만들었다가 사용성 검토 뒤 2종으로 합침 — `docs/superpowers/plans/2026-11-10-phase3b-image-ux.md`)
- **완료 기준:** 도구별 fixtures 테스트(출력 포맷, 크기, 목표 KB 오차 ±5% 이내, 메타데이터 제거 확인), UI에서 폴더 일괄·단일 처리 모두 확인

### Phase 4. 추가 설치 구조
- `addon-abi`(C ABI + `export_tool!`), DLL 로더, 설치·업데이트·제거, sha256·minisign 확인, ABI 버전 처리, GitHub Actions 릴리스 워크플로
- 첫 추가 도구로 `image.base64`, `image.icon` (2026-11-11 변경, 처음 계획은 `image.svg`)
- **완료 기준:** 로컬 index(debug 전용)로 설치 → 실행 → 업데이트 → 제거, 서명 틀린 DLL 거부, 패닉하는 더미 DLL이 앱을 죽이지 않음, `image.base64`·`image.icon`을 내장/추가 두 방식으로 빌드해도 둘 다 동작

### Phase 5. 추가 설치 이미지 5종
- `image.watermark`, `image.svg`, `image.gif_optimize`, `image.sprite`
- **완료 기준:** 도구별 테스트, 앱 아이콘 출력 세트가 4장 명세와 일치, 비교 화면 3가지 보기 동작, 실제 Releases에서 설치 확인

### Phase 6. 배포
- Windows 설치 파일(NSIS 또는 MSI), 앱 아이콘, 버전 정보
- **완료 기준:** 깨끗한 Windows에서 설치 → 기본 7종은 네트워크 없이 동작, 추가 도구는 설치 후 네트워크를 끊고 동작

### PDF 5종 (2026-11-11, Phase 5보다 먼저)
- PDF 5종을 추가 설치 도구로 만든다 (5장 명세, [PDF 도구 설계](superpowers/specs/2026-11-11-pdf-tools-design.md)). 공통 기능: core Combine 실행, 미리보기 없음(`x-preview: "none"`), 입력 카드 끌어서 순서 바꾸기.

### 영상 1종 (2026-11-11)
- `video.compress`를 추가 설치 도구로 만들고 `영상` 탭(PDF 다음)을 둔다 (6장 명세).
- **완료 기준:** 임시 폴더 테스트 — 해상도 줄이기·소리 빼기·목표 용량 맞추기·영상 아닌 파일 오류·취소, 원본 그대로

### 영상 4종 · AI 이미지 2종 (2026-11-12)
- `video.convert`·`video.to_gif`·`video.from_gif`·`video.thumbnail`(공용 `video-common`으로 Media Foundation 코드 이동), `image.upscale`·`image.bg_remove`(AI, 목록에 AI 뱃지)를 추가 설치 도구로 만든다 (4장·6장 명세).
- **완료 기준:** 도구별 임시 폴더 테스트(정상 변환·옵션별 결과·잘못된 입력 오류·취소·원본 그대로), video-compress·gif-optimize 회귀 없음, AI 도구는 모델 없는 빌드에서도 workspace 검사 통과·DLL 하나로 로드

### 이후 (이미지 완료 뒤)
- 웹·폰트 1종, 기타 5종을 추가 설치 도구로 만든다 (6장 명세).

---

## 9. Claude Code 작업 규칙

아래 내용을 저장소 루트의 `CLAUDE.md`로 저장한다.

```markdown
# 작업 규칙

- docs/plan.md와 docs/superpowers/specs/2026-11-10-image-first-addon-design.md가 기준 문서다. 지시받은 Phase 범위 밖의 기능은 만들지 않는다.
- UI 작업은 docs/UI-SYSTEM.md를 따른다. 화면 코드에 #hex, bg-[…], text-gray-500 같은 토큰 밖 색을 쓰지 않는다.
- AI/머신러닝 모델을 쓰는 기능은 사용자가 지시한 것(`image.upscale` Real-ESRGAN, `image.bg_remove` BiRefNet, 2026-11-12)만 둔다. 모델은 도구 DLL 안에 넣고 실행 중에 받지 않으며, 카탈로그에 `"ai": true`(목록의 AI 뱃지)를 단다.
- 네트워크는 app/src-tauri의 추가 도구 설치 모듈과 앱 자동 업데이트(tauri-plugin-updater, GitHub Releases `cksrlcks/waple-tools`만, 서명 확인)에서만 쓴다. 그 밖의 곳에 네트워크 크레이트, Tauri http 권한, 외부 CDN 리소스를 추가하지 않는다.
- 원본 파일을 삭제하거나 덮어쓰는 코드는 금지한다. 도구는 out_dir에만 쓰고, 파일 이동은 반드시 core의 배치 엔진을 거친다.
- 작업 시작 전 변경 계획을 짧게 제시하고, 끝나면 아래를 모두 통과시킨다.
  - cargo fmt --check
  - cargo clippy --workspace -- -D warnings
  - cargo test --workspace
- 사용자 파일을 다루는 코드는 임시 폴더 기반 테스트를 함께 작성한다. 테스트용 샘플은 tests/fixtures에 둔다.
- 새 도구는 crates/tools/<카테고리>-<이름>/ 크레이트 하나로 만들고 catalog/tools.json에 추가한다.
  기본 내장이면 app/src-tauri의 builtin()에 등록, 추가 설치면 export_tool! 한 줄로 DLL을 내보낸다.
  도구 전용 UI 코드는 작성하지 않고 옵션 스키마와 결과 블록으로 해결한다. 불가능한 UI가 필요하면 먼저 제안한다.
- 의존성을 추가할 때는 이유와 라이선스를 커밋 메시지에 적는다. GPL/AGPL 라이선스는 사용 전에 확인을 받는다.
- 화면 문구, 옵션 설명, 오류 메시지는 한국어로 작성한다.
- Phase가 끝나면 PLAN.md의 완료 기준을 하나씩 확인한 결과를 보고한다.
```

### 지시 예시

- "docs/plan.md를 읽고 Phase 0을 진행해줘."
- "Phase 1의 완료 기준을 테스트로 검증하고 결과를 알려줘."
- "Phase 3에서 `image.optimize`만 먼저 구현해줘."
- 새 기능 추가 시: "plan.md 4장에 `image.xxx` 명세를 추가했어. 추가 설치 도구로 구현해줘."