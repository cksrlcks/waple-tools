# Phase 3 기본 내장 이미지 도구 7종 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `image.webp`, `image.convert`, `image.resize`, `image.compress`, `image.strip_meta`, `image.rotate`, `image.crop`을 도구 크레이트로 만들어 앱에 기본 내장한다.

**Architecture:** 공용 크레이트 `waple-tools-image-common`이 이미지 읽기(EXIF 방향 반영)·포맷별 인코딩(JPEG=mozjpeg, PNG=image+oxipng, WebP=libwebp)·메타데이터(EXIF·ICC 읽기/붙이기/GPS 제거)를 맡고, 도구 크레이트는 옵션 구조체 + `run` 하나만 갖는다. 도구는 Phase 0 계약대로 `out_dir`에 결과 파일 하나를 쓰고 이름을 돌려준다. 원본 이동·백업은 배치 엔진이 한다.

**Tech Stack:** `image 0.25`(jpeg·png·webp·gif·bmp·tiff 디코딩), `mozjpeg 0.10`, `webp 0.3`, `oxipng 9`(`default-features = false, features = ["parallel"]`), `img-parts 0.3`, `kamadak-exif 0.6`, `schemars 1` — 이 PC에서 cmake·nasm 없이 빌드 확인함

**Spec:** [docs/plan.md](../../plan.md) 4장 표 1·3·4·5·6·7·8번과 공통 문단, 8장 Phase 3, [설계 문서](../specs/2026-11-10-image-first-addon-design.md) 4장

## Global Constraints

- 도구는 `out_dir`에만 쓴다. 결과 파일 하나, 이름은 `<원본 stem>.<새 확장자>`.
- 옵션 구조체는 `#[derive(Deserialize, JsonSchema)]`, `#[serde(default, rename_all = "camelCase")]`, 필드 설명은 한국어 doc 주석(폼 라벨이 된다), enum 값은 `rename_all = "camelCase"` + 각 값에 한국어 doc 주석(선택 상자 라벨).
- 범위가 있는 숫자는 `#[schemars(range(min = .., max = ..))]` → 폼에서 슬라이더.
- EXIF 방향은 읽을 때 픽셀에 반영한다. 메타데이터를 유지할 때는 EXIF 방향 값을 1로 바꿔 붙인다(두 번 회전 방지).
- 메타데이터 공통 옵션 `metadata`: 기본 "지우기"(plan.md 4장). 앱 설정 "기본 메타데이터 처리"가 이 기본값을 바꾼다(Task 8).
- 오류 문구 한국어: 읽기 실패 `"이미지를 읽을 수 없습니다."`, 지원 안 함 `"지원하지 않는 이미지 형식입니다."`.
- 도구 id·이름은 `catalog/tools.json`과 같아야 한다(`catalog_matches_builtin` 테스트가 검사).
- 의존성 추가 커밋에 이유·라이선스: image(MIT/Apache-2.0), mozjpeg(IJG·BSD·zlib), webp(MIT/Apache-2.0, libwebp BSD-3), oxipng(MIT), img-parts(MIT/Apache-2.0), kamadak-exif(BSD-2).
- 끝날 때마다 `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## 이번 계획에서 정한 것

- **AVIF 입력:** 기본 내장 도구는 AVIF를 읽지 않는다. AVIF 디코더(dav1d)는 크고 C 빌드가 필요해 기본 앱 용량을 늘린다 → 추가 설치 도구 `image.avif`가 생길 때 그 도구가 가져온다. 입력 확장자: `jpg, jpeg, png, webp, gif, bmp, tif, tiff`.
- **GIF 입력:** 첫 프레임만 쓴다(움직이는 GIF 처리는 추가 설치 도구 `image.gif_optimize`).
- **JPG 무손실 회전 (사용자 요청으로 포함):** mozjpeg-sys에 들어 있는 jpegtran 변환 코드(`vendor/transupp.c`)를 `image-rotate` 크레이트의 build.rs에서 함께 컴파일하고, 작은 C 래퍼 하나(`lt_jpeg_transform`)로 부른다. `perfect` 모드라 MCU 경계가 맞지 않아 손실 없이 못 돌리는 이미지, EXIF 방향이 1이 아닌 이미지는 품질 95로 다시 저장한다.
- **PNG 색상 수 줄이기 (사용자 요청으로 포함):** `quantette 0.6`(MIT/Apache-2.0, Oklab·디더링)로 팔레트 PNG를 만든 뒤 oxipng로 최적화. 압축 옵션 "PNG 색상 수"(0=줄이지 않음, 2~256). imagequant는 GPL이라 쓰지 않는다. PNG에 "목표 KB" 모드를 고르면 색상 수 옵션만 적용한다.
- **리사이즈:** `image`의 Lanczos3 (fast_image_resize는 필요해지면).

## Review Focus

1. 휴대폰 사진(EXIF 방향 6 = 90° 돌아간 JPEG)을 변환 → 결과가 눈에 보이는 방향 그대로, 메타데이터 유지 시에도 다시 돌아가지 않음. → Task 1 `load_applies_orientation`, Task 2 `keep_metadata_resets_orientation`
2. 투명 PNG를 JPG로 변환 → 투명 부분이 검게 되지 않고 지정한 배경색(기본 흰색). → Task 3 `transparent_png_to_jpg_uses_background`
3. 이미지가 아닌 파일·깨진 파일에 확장자만 `.jpg` → 그 파일만 "이미지를 읽을 수 없습니다."로 실패, 패닉 없음. → Task 1 `load_rejects_broken_file`
4. 목표 KB가 너무 작아 가장 낮은 품질로도 못 맞춤 → 가장 작은 결과를 내고 실패하지 않음(크기 비교는 엔진 몫). → Task 5 `target_kb_unreachable_returns_smallest`
5. 확대 금지가 켜진 상태에서 원본보다 큰 크기로 리사이즈 → 원본 크기 그대로(다시 인코딩만). → Task 4 `no_upscale_keeps_size`

---

## 파일 구조

```
crates/tools/image-common/         # waple-tools-image-common
  src/lib.rs                       # pub use
  src/format.rs                    # Fmt (jpeg/png/webp/gif/bmp/tiff), from_path, ext, SUPPORTED_EXTS
  src/load.rs                      # load(): 디코딩 + EXIF 방향 반영 + Meta 추출
  src/encode.rs                    # encode(): Fmt별 인코딩, flatten()
  src/meta.rs                      # Meta, MetaMode, with_meta(), exif_reset_orientation(), exif_without_gps()
  src/output.rs                    # write_output()
  src/testkit.rs                   # (feature "testkit") photo(), alpha_png(), jpeg_with_exif()
crates/tools/image-webp/  image-convert/  image-resize/  image-compress/
crates/tools/image-strip-meta/  image-rotate/  image-crop/
crates/builtin/src/lib.rs          # builtin()에 7종 추가
crates/builtin/src/settings.rs     # keep_metadata: bool (기본 false)
catalog/tools.json                 # 7종 builtin: true
app/src/...                        # 설정 화면 행 + 기본값 적용 (Task 8)
```

---

### Task 1: 공용 크레이트 `image-common`

**Interfaces — Produces** (`waple_tools_image_common`):
- `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Fmt { Jpeg, Png, Webp, Gif, Bmp, Tiff }` — `pub fn from_path(p: &Path) -> Option<Fmt>`(확장자 소문자, jpg/jpeg, tif/tiff), `pub fn ext(self) -> &'static str`(`"jpg"`, `"png"`, `"webp"`, `"gif"`, `"bmp"`, `"tiff"`)
- `pub const INPUT_EXTS: &[&str] = &["jpg","jpeg","png","webp","gif","bmp","tif","tiff"]`
- `#[derive(Clone, Default, Debug)] pub struct Meta { pub exif: Option<Vec<u8>>, pub icc: Option<Vec<u8>> }`
- `pub struct Loaded { pub image: DynamicImage, pub format: Fmt, pub meta: Meta }`
- `pub fn load(path: &Path) -> Result<Loaded, ToolError>` — 포맷은 내용으로 판별(`image::guess_format`), EXIF·ICC는 `img-parts`로 JPEG·PNG·WebP에서 읽음, EXIF 방향(`kamadak-exif`, 태그 0x0112) 2~8을 `DynamicImage`의 `fliph/rotate90/…`로 반영. 실패 → `ToolError::Failed("이미지를 읽을 수 없습니다.")`
- `#[derive(Clone, Copy)] pub struct Quality { pub quality: u8, pub lossless: bool }`
- `pub fn encode(img: &DynamicImage, fmt: Fmt, q: Quality) -> Result<Vec<u8>, ToolError>` — Jpeg: `mozjpeg`(RGB8, 알파가 있으면 호출자가 먼저 `flatten`), Png: `image` PNG 인코더 → `oxipng::optimize_from_memory`(preset 2, 메타데이터 청크 제거 `StripChunks::Safe`), Webp: `webp::Encoder`(`lossless`면 무손실, 아니면 `quality`), Gif/Bmp/Tiff: `image`의 기본 인코더
- `pub fn flatten(img: &DynamicImage, bg: [u8; 3]) -> DynamicImage` — 알파를 배경색 위에 합성해 RGB8로
- `#[derive(Clone, Copy, Default, Deserialize, JsonSchema, PartialEq, Eq, Debug)] #[serde(rename_all = "camelCase")] pub enum MetaMode { /// 지우기 #[default] Strip, /// 유지 Keep }`
- `pub fn with_meta(bytes: Vec<u8>, fmt: Fmt, meta: &Meta) -> Result<Vec<u8>, ToolError>` — JPEG·PNG·WebP만 `img-parts`로 EXIF(`exif_reset_orientation` 적용 후)·ICC 삽입, 그 밖의 포맷은 그대로
- `pub fn exif_reset_orientation(exif: &[u8]) -> Vec<u8>` — IFD0의 0x0112 값을 1로 바꾼 사본(태그가 없으면 그대로). `kamadak-exif`로 읽고 `exif::experimental::Writer`로 다시 쓴다.
- `pub fn exif_without_gps(exif: &[u8]) -> Result<Vec<u8>, ToolError>` — GPS IFD 필드를 뺀 EXIF
- `pub fn write_output(out_dir: &Path, input: &Path, fmt: Fmt, bytes: &[u8]) -> Result<PathBuf, ToolError>` — `out_dir/<stem>.<ext>`에 쓰고 상대 이름(`<stem>.<ext>`) 반환
- `testkit`(feature): `photo(w: u32, h: u32) -> DynamicImage`(그라디언트 + 고정 시드 노이즈 — 압축률이 품질에 반응하도록), `alpha_png(path, w, h)`(왼쪽 절반 완전 투명), `jpeg_with_exif(path, w, h, orientation: u16, gps: bool)`(방향·GPS가 든 EXIF를 붙인 JPEG)

- [ ] **Step 1: 실패하는 테스트** (`#[cfg(test)]`, testkit 사용)
  - `fmt_from_path`: `a.JPG`→Jpeg, `b.tif`→Tiff, `c.txt`→None
  - `load_applies_orientation`: 40×20 `jpeg_with_exif(orientation 6)` → `load` 결과 크기 20×40 (Review Focus 1)
  - `load_reads_exif_and_keeps_gps`: `jpeg_with_exif(gps: true)` → `meta.exif`가 있고 kamadak-exif로 GPS 필드가 읽힘
  - `load_rejects_broken_file`: 내용이 `"not image"`인 `x.jpg` → `Err(Failed("이미지를 읽을 수 없습니다."))` (Review Focus 3)
  - `encode_roundtrip_each_format`: `photo(64, 48)`을 Jpeg/Png/Webp(손실·무손실)로 인코딩 → `image::load_from_memory` 크기 64×48, `guess_format`이 맞음
  - `jpeg_quality_affects_size`: 품질 30 결과 < 품질 90 결과
  - `flatten_uses_background`: 완전 투명 픽셀 → 배경색 `[255, 0, 0]`
  - `exif_reset_orientation_sets_one`, `exif_without_gps_removes_gps`: 처리 후 kamadak-exif로 확인
  - `with_meta_inserts_exif_into_png_and_webp`: 인코딩한 PNG·WebP에 EXIF를 붙인 뒤 img-parts로 다시 읽으면 EXIF가 있고 `image`로 디코딩 가능
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + clippy
- [ ] **Step 5: Commit** — 본문에 의존성 6개 이유·라이선스

### Task 2: `image.webp` (WebP 변환)

**Interfaces:** 크레이트 `crates/tools/image-webp` (`waple-tools-tool-image-webp`), `pub fn tool() -> Box<dyn Tool>`. meta: id `image.webp`, name `WebP 변환`, category `image`, description은 카탈로그와 같게, extensions `INPUT_EXTS`.
- 옵션 `WebpOptions { /// 품질 #[schemars(range(min = 0, max = 100))] quality: u8 = 80, /// 무손실 lossless: bool = false, /// 메타데이터 metadata: MetaMode = Strip }`
- 동작: load → encode(Webp) → `metadata == Keep`이면 `with_meta` → `write_output(.., Fmt::Webp, ..)`

- [ ] **Step 1: 실패하는 테스트**
  - `converts_png_to_webp`: `photo` PNG 입력 → 결과 이름 `a.webp`, `guess_format` WebP, 크기 같음
  - `lossless_is_pixel_exact`: 무손실 → 디코딩 픽셀이 원본과 같음
  - `strip_by_default`: EXIF 붙은 JPEG 입력 → 결과에 EXIF 없음
  - `keep_metadata_resets_orientation`: 방향 6 JPEG + `metadata: keep` → 결과 EXIF 방향 1, 결과 크기는 회전 반영(세로) (Review Focus 1)
  - `schema_has_slider_for_quality`: `options_schema()`의 `properties.quality`에 `minimum: 0`, `maximum: 100`, `default: 80`
- [ ] **Step 2:** FAIL → **Step 3:** 구현 → **Step 4:** PASS
- [ ] **Step 5: Commit** — `feat: WebP 변환 도구`

### Task 3: `image.convert` (포맷 변환)

- 옵션 `ConvertOptions { /// 출력 포맷 format: OutFmt = Png (/// JPG Jpg, /// PNG Png, /// WebP Webp), /// JPG 품질 #[range 1..=100] jpgQuality: u8 = 90, /// 투명 배경 대체 색 (JPG) #[schemars(extend("format" = "color"))] background: String = "#ffffff", metadata: MetaMode }`
- 동작: load → JPG 출력이면 `flatten(background)` → encode(WebP는 품질 90 손실) → meta → write
- `background` 파싱 실패(`#rrggbb` 아님) → `ToolError::InvalidOptions("배경색은 #rrggbb 형식이어야 합니다.")`

- [ ] **Step 1: 실패하는 테스트**
  - `png_to_jpg_changes_extension`: 결과 `a.jpg`, JPEG
  - `transparent_png_to_jpg_uses_background`: `alpha_png` + background `#ff0000` → 결과 (0,0) 픽셀이 빨강에 가까움(각 채널 ±10) (Review Focus 2)
  - `jpg_to_png_is_png`
  - `bad_background_is_invalid_options`
  - `schema_marks_background_as_color`: `properties.background.format == "color"`
- [ ] **Step 2–4** → **Step 5: Commit** — `feat: 포맷 변환 도구`

### Task 4: `image.resize` (리사이즈)

- 옵션 `ResizeOptions { /// 기준 mode: Mode = LongEdge (/// 가로 기준 Width, /// 세로 기준 Height, /// 긴 변 기준 LongEdge, /// 정확한 크기 Exact), /// 가로(px) — 긴 변 기준일 때는 긴 변 길이 #[schemars(range(min = 1))] width: u32 = 1920, /// 세로(px) #[schemars(range(min = 1))] height: u32 = 1080, /// 맞춤 (정확한 크기) fit: Fit = Contain (/// 비율 유지 Contain, /// 늘리기 Stretch, /// 크롭 채우기 Cover), /// 확대 금지 noUpscale: bool = true, /// 출력 포맷 format: KeepOr = Keep (/// 원본 유지 Keep, /// JPG Jpg, /// PNG Png, /// WebP Webp), /// 품질 (JPG·WebP) #[range 1..=100] quality: u8 = 90, metadata: MetaMode }`
  - `width`/`height`는 최솟값만 둬서 숫자 칸이 되게 한다(슬라이더는 범위가 너무 넓다). 실행 시 20000을 넘으면 20000으로 자른다.
- 목표 크기 계산은 순수 함수 `fn target_size(w, h, opts) -> (u32, u32)`로 분리:
  - Width: 가로=width, 세로는 비율. Height: 세로=height, 가로는 비율. LongEdge: 긴 변=width, 짧은 변은 비율. Exact: (width, height).
  - `noUpscale`이고 목표가 원본보다 크면(Width/Height/LongEdge는 기준 변, Exact는 둘 다) 원본 크기.
  - Exact + Contain: 비율 유지로 상자 안에 맞춤(결과가 상자보다 작을 수 있음), Stretch: 정확히, Cover: 비율 유지로 덮은 뒤 가운데 크롭.
- 출력 포맷 Keep이면 입력 포맷(Gif/Bmp/Tiff 포함)으로, JPG면 흰 배경 flatten.

- [ ] **Step 1: 실패하는 테스트** — `target_size` 단위 테스트: 4000×3000 Width 1920 → 1920×1440; Height 1080 → 1440×1080; 3000×4000 LongEdge 1920 → 1440×1920; Exact Contain 1000×1000 → 1000×750; Exact Stretch → 1000×1000; `no_upscale_keeps_size`: 800×600 Width 1920 → 800×600 (Review Focus 5); 도구 실행 `cover_crops_to_exact_box`: 400×300 → Exact Cover 200×200 결과 200×200; `keep_format_keeps_extension`(png→png)
- [ ] **Step 2–4** → **Step 5: Commit** — `feat: 리사이즈 도구`

### Task 5: `image.compress` (이미지 압축)

- 옵션 `CompressOptions { /// 방식 mode: Mode = Quality (/// 품질 지정 Quality, /// 목표 용량 TargetKb), /// 품질 #[range 1..=100] quality: u8 = 75, /// 목표 용량(KB) #[schemars(range(min = 1))] targetKb: u32 = 200, metadata: MetaMode }`
- 옵션 추가: `/// PNG 색상 수 (0은 줄이지 않음) #[range 0..=256] pngColors: u16 = 0`
- 동작(포맷 유지): JPG → mozjpeg, WebP → 손실 재인코딩, PNG → `pngColors`가 2 이상이면 quantette로 팔레트 PNG(디더링) 후 oxipng(preset 4), 0이면 oxipng 무손실만, GIF·BMP·TIFF → `ToolError::Failed("이 형식은 압축을 지원하지 않습니다.")`
- 목표 KB 탐색: 순수 함수 `fn search_quality(target: usize, encode: impl Fn(u8) -> Result<Vec<u8>, ToolError>) -> Result<Vec<u8>, ToolError>` — 품질 1~100 이진 탐색으로 `size <= target`인 가장 높은 품질의 결과. 품질 1로도 크면 품질 1 결과 반환 (Review Focus 4). 인코딩 횟수 ≤ 8.

- [ ] **Step 1: 실패하는 테스트**
  - `search_quality_hits_target_within_5_percent`: `photo(1024, 768)` JPEG에서 품질 40과 80의 크기 중간값을 목표로 → 결과 크기 ≤ 목표, 그리고 `(목표 - 크기) / 목표 ≤ 0.05`
  - `search_quality_encodes_at_most_8_times`: 호출 횟수 세기
  - `target_kb_unreachable_returns_smallest`: 목표 1 byte → 품질 1 결과, `Ok` (Review Focus 4)
  - `png_is_lossless`: `pngColors: 0` → 압축 후 디코딩 픽셀 동일, 크기 ≤ 원본
  - `png_colors_reduces_palette`: `photo` PNG + `pngColors: 16` → 결과 디코딩 시 서로 다른 색 ≤ 16, 크기 < 무손실 결과
  - `gif_is_rejected`
- [ ] **Step 2–4** → **Step 5: Commit** — `feat: 이미지 압축 도구 (품질·목표 용량)`

### Task 6: `image.strip_meta` (메타데이터 제거)

- 옵션 `StripOptions { /// 제거 범위 scope: Scope = All (/// 전체 All, /// GPS만 GpsOnly), /// 색상 프로파일 유지 keepIcc: bool = true }`
- 동작: **다시 인코딩하지 않는다.** JPEG·PNG·WebP는 `img-parts`로 원본 바이트를 열어 EXIF 제거(All) 또는 `exif_without_gps`로 교체(GpsOnly), `keepIcc`가 false면 ICC도 제거, 그대로 저장. 그 밖의 형식은 `Failed("이 형식은 메타데이터 제거를 지원하지 않습니다.")`. JPEG의 XMP·주석(APP1 XMP, COM)도 All이면 제거.
- extensions: `jpg, jpeg, png, webp`

- [ ] **Step 1: 실패하는 테스트**
  - `all_removes_exif_and_keeps_pixels_bytes`: EXIF JPEG → 결과에 EXIF 없음, 이미지 데이터(SOS 이후 바이트) 원본과 동일
  - `gps_only_keeps_other_exif`: 결과 EXIF에 GPS 없음, Orientation 등 다른 태그 남음
  - `keep_icc_false_removes_icc`(ICC 붙인 PNG)
  - `bmp_is_rejected`
- [ ] **Step 2–4** → **Step 5: Commit** — `feat: 메타데이터 제거 도구`

### Task 7: `image.rotate` · `image.crop`

- rotate 옵션 `RotateOptions { /// 회전 angle: Angle = R90 (/// 회전 안 함 None, /// 90° R90, /// 180° R180, /// 270° R270), /// 좌우 뒤집기 flipH: bool, /// 상하 뒤집기 flipV: bool, /// 품질 (JPG·WebP) #[range 1..=100] quality: u8 = 95, metadata: MetaMode }` — 회전 후 뒤집기, 포맷 유지. 아무 변화도 없으면(None·뒤집기 없음) `InvalidOptions("회전이나 뒤집기를 하나 이상 고르세요.")`.
- crop 옵션 `CropOptions { /// 방식 mode: Mode = Ratio (/// 비율 Ratio, /// 픽셀 Pixels), /// 비율 ratio: Ratio = R1x1 (/// 1:1, /// 4:3, /// 3:4, /// 16:9, /// 9:16, /// 3:2, /// 2:3), /// 가로(px) #[range(min=1)] width: u32 = 1000, /// 세로(px) #[range(min=1)] height: u32 = 1000, /// 기준 위치 anchor: Anchor = Center (/// 가운데, /// 위, /// 아래, /// 왼쪽, /// 오른쪽, /// 왼쪽 위, /// 오른쪽 위, /// 왼쪽 아래, /// 오른쪽 아래), /// 품질 quality: u8 = 95, metadata: MetaMode }`
  - 순수 함수 `fn crop_rect(w, h, opts) -> (x, y, cw, ch)`: Ratio는 원본 안에 들어가는 최대 비율 상자, Pixels는 min(원본)으로 자름, anchor로 위치.

- JPG 무손실: `build.rs`가 `DEP_JPEG_INCLUDE`(mozjpeg-sys가 내보냄)로 `transupp.c` + `src/lt_transform.c`를 `cc`로 컴파일. C 함수 `int lt_jpeg_transform(const unsigned char* in, unsigned long in_len, int op, int copy_all_markers, unsigned char** out, unsigned long* out_len)` — op: 1=좌우, 2=상하, 3=90°, 4=180°, 5=270°; `perfect = TRUE`, 실패(완벽하지 않음·오류)는 0이 아닌 값; 메모리는 `lt_free`로 해제. setjmp로 libjpeg 오류를 잡는다. Rust 쪽 `fn lossless_jpeg(bytes, ops) -> Option<Vec<u8>>`는 회전과 뒤집기를 차례로 적용하고 하나라도 실패하면 None → 다시 저장 경로.
- [ ] **Step 1: 실패하는 테스트** — rotate: `r90_swaps_dimensions`(40×20→20×40), `flip_h_mirrors_pixel`(왼쪽 위 빨강 → 오른쪽 위), `noop_is_invalid`, `jpeg_r90_is_lossless_when_mcu_aligned`(64×32 JPEG → 90° → 32×64이고, 다시 270°로 돌리면 원본 JPEG의 디코딩 픽셀과 정확히 같음), `jpeg_unaligned_falls_back`(65×33 JPEG → 90° 결과 33×65, 오류 없음); crop: `crop_rect` 표 — 400×300 1:1 Center → (50,0,300,300), 16:9 Top → (0,0,400,225), Pixels 500×100 BottomRight → (0,200,400,100); 실행 `crop_writes_box_size`
- [ ] **Step 2–4** → **Step 5: Commit** — `feat: 회전·뒤집기, 크롭 도구`

### Task 8: 내장 등록 · 기본 메타데이터 설정 · 수동 확인

**Files:** `crates/builtin/Cargo.toml`, `crates/builtin/src/{lib,settings}.rs`, `catalog/tools.json`(7종 `builtin: true`, 이름이 각 도구 meta와 같아야 함), `app/src/entities/settings/model/types.ts`, `app/src/widgets/settings-panel/*`, `app/src/widgets/tool-workspace/ui/file-tool-workspace.tsx`

- `builtin()`: `[dev-copy, image-webp, image-convert, image-resize, image-compress, image-strip-meta, image-rotate, image-crop]`
- `Settings.keep_metadata: bool`(기본 false) ↔ TS `keepMetadata: boolean`; 설정 화면 행 "기본 메타데이터 처리"(선택: 지우기/유지)
- 작업 화면: 도구에 `metadata` 필드가 있고 `lastOptions`에 저장된 값이 없으면, 초기 옵션의 `metadata`를 설정값(`keep`/`strip`)으로
- 확인:
  - `cargo test --workspace` (카탈로그 정합성 테스트가 7종 이름 일치를 검사)
  - `npm run tauri dev`(사용자 확인): JPG 폴더 → WebP 변환 → 결과·용량 감소 확인 → 되돌리기; 리사이즈 긴 변 1920; 압축 목표 KB; 메타데이터 제거 후 탐색기 속성에 GPS 없음
  - CLI로 실제 파일 확인: `cargo run -p waple-tools-cli -- run image.webp <폴더>` → 결과 webp, `original/`에 원본

- [ ] **Step 1:** 등록·카탈로그·설정 구현 → `cargo test --workspace`, `npx tsc --noEmit`, `npx vitest run` PASS
- [ ] **Step 2:** CLI로 샘플 폴더(테스트 키트로 만든 JPG 3장) 실행·되돌리기 확인
- [ ] **Step 3: Commit** — `feat: 기본 내장 이미지 도구 7종 등록, 기본 메타데이터 설정`

---

## Phase 3 완료 기준 대조 (plan.md 8장, 이미지 핵심)

| 기준 | 테스트 |
| --- | --- |
| 출력 포맷 | 각 도구 `*_changes_extension` / `guess_format` 단언 |
| 크기 | `target_size` 표, `crop_rect` 표, `r90_swaps_dimensions` |
| 목표 KB 오차 ±5% | `search_quality_hits_target_within_5_percent` |
| 메타데이터 제거 | `strip_by_default`, `all_removes_exif_…`, `gps_only_keeps_other_exif` |
| UI 폴더 일괄·단일 | Task 8 수동 확인 (사용자) |
