# Phase 3b 이미지 도구 재구성 · 사용성 개선 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 기본 내장 이미지 도구 7종을 사용자가 정한 2종(`image.optimize` 이미지 최적화, `image.edit` 자르기·회전·여백)으로 합치고, 실행 전 썸네일과 원본 ↔ 결과 비교(슬라이더·나란히 보기·용량 변화), 옵션 연결(무손실이면 품질 비활성 등), 창 맨 위 카테고리 탭을 만든다.

**Architecture:** 도구 계약·배치 엔진은 그대로. `image-common`을 그대로 쓰는 새 크레이트 2개가 옛 크레이트 7개를 대신한다(로직은 옛 크레이트에서 옮김 — JPG 무손실 회전 C 래퍼 포함). 미리보기는 공통 기능: Rust `preview_tool`이 고른 파일 하나에 도구를 임시 폴더로 실행해 원본·결과를 화면용 JPEG로 돌려주고, 프론트 비교 화면이 보여 준다. 옵션 연결은 스키마 확장 키 `x-enabled-when`.

**Tech Stack:** 기존과 같음 + `base64 0.22`

**Spec:** 사용자가 정한 목록(이 대화) → [docs/plan.md](../../plan.md) 4장(8종으로 갱신됨)·8장 Phase 3, [설계 문서](../specs/2026-11-10-image-first-addon-design.md) 4장, [docs/UI-SYSTEM.md](../../UI-SYSTEM.md)

## Global Constraints

- 도구 전용 UI 코드는 만들지 않는다. 미리보기·비교·옵션 연결·탭은 공통 기능.
- 미리보기는 원본을 바꾸지 않고 작업 기록도 남기지 않는다(임시 폴더 실행 후 삭제).
- 화면용 이미지는 Rust에서 만들어 data URL로 보낸다(asset 프로토콜·파일 권한을 넓히지 않음, TIFF·BMP도 보임).
- 옛 크레이트(`image-webp`, `image-convert`, `image-compress`, `image-resize`, `image-strip-meta`, `image-rotate`, `image-crop`)는 로직을 새 크레이트로 옮긴 뒤 삭제. 테스트도 같이 옮긴다(되돌아가는 기능이 없게).
- 한국어 문구, 토큰 색만, 페이드 외 애니메이션 없음, 파일 하나 = 컴포넌트 하나, FSD.
- 끝날 때마다 `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npx tsc --noEmit`, `npx vitest run`, `npx prettier --check src`.

## 정한 것

- 빠진 도구: 메타데이터 제거(옵션으로 남음), 반응형 이미지 세트. 이미지 비교는 공통 비교 화면으로 대신.
- `image.optimize`는 단계 스위치(크기·포맷·용량)로 필요한 단계만 쓴다. 기본은 용량 줄이기만 켜짐 (사용자 결정).
- 카테고리 탭은 창 맨 위 탭 막대(사이드바 폭 240px에 5~6개 탭이 안 들어감). 탭을 고르면 사이드바는 그 카테고리 도구만, 검색도 그 탭 안에서. 즐겨찾기는 각 탭 목록 맨 위에 그 탭의 즐겨찾기만.
- 지금 "미리보기" 버튼(dry-run)은 "대상 확인"으로 이름을 바꾼다.

## 화면 구성

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ [이미지] [PDF] [웹·폰트] [기타] [개발용]              ← 탭 막대 h-9 bg-titlebar │
├──────────┬──────────── 왼쪽 (입력·옵션) ───────────┬─────── 오른쪽 (미리보기) ───┤
│ 도구 검색 │ 이미지 최적화                           │ [슬라이더 | 나란히]          │
│ ★ …      │ [드롭 영역   파일 선택 · 폴더 선택]     │ ┌──────────────────────────┐ │
│ 이미지   │ ┌──┐ photo1.jpg  1.2MB  대기  ✕ (선택) │ │ 원본    ┃     결과        │ │
│  최적화  │ ┌──┐ photo2.jpg  1.1MB  대기  ✕        │ └──────────────────────────┘ │
│  자르기… │ ── 크기 ── ── 포맷 ── ── 품질 ──       │ 1.15 MB → 160 KB (−86%)      │
│  워터마크│ (안 쓰는 칸은 흐리게)                   │ 1600×1200 → 1200×630         │
│ ──────── │ ── 저장 방식 ──                         │                              │
│ ⚙ 설정   │ [대상 확인] [실행]                      │                              │
└──────────┴─────────────────────────────────────────┴──────────────────────────────┘
```

## Review Focus

1. 옵션을 빠르게 여러 번 바꿈 → 마지막 옵션의 결과만 보인다. → Task 6: queryKey에 옵션 포함 + 300ms 디바운스 + `keepPreviousData` (수동 확인)
2. 수천만 화소 사진 → 화면용 이미지는 긴 변 1600px, data URL 수 MB 이하. → Task 5 `display_image_is_downscaled`
3. 미리보기 중 도구 오류 → 비교 화면에 오류 문구, 앱 정상. → Task 5 `preview_reports_tool_error`
4. 미리보기가 원본·작업 기록을 건드리지 않음. → Task 5 `preview_leaves_no_trace`
5. 비활성 옵션 값이 남아 있어도 결과가 맞음(무손실이면 품질 무시, 크기 스위치가 꺼져 있으면 가로·세로 무시). → Task 1 `lossless_ignores_quality`, `resize_off_ignores_width`

---

### Task 1: `image.optimize` (크기 → 포맷 → 품질)

**Files:** Create `crates/tools/image-optimize/`; 로직·테스트를 `image-resize`, `image-convert`, `image-compress`, `image-webp`에서 옮긴 뒤 그 4개 삭제

**처리 방식 — 단계 스위치 3개** (사용자 결정): 크기 바꾸기·포맷 바꾸기·용량 줄이기를 각각 켜고 끈다. 꺼진 단계는 건너뛰고 그 단계 칸은 흐려진다. 기본값은 **용량 줄이기만 켜짐**. 하나도 안 켜면 실행할 수 없다.

| 켠 단계 | 동작 |
|---|---|
| 용량 줄이기만 | 크기·포맷 그대로 압축 |
| 크기만 | 크기만 바꾸고 화질 최대 유지(JPG·WebP 품질 95, PNG 무손실) |
| 포맷만 | 포맷만 바꾸고 화질 최대 유지(WebP 무손실 선택 가능) |
| 여러 개 | 크기 → 포맷 → 용량 순서 |

**Interfaces — 옵션 `OptimizeOptions`** (`#[serde(default, rename_all = "camelCase")]`, 각 필드 한국어 doc 주석 = 폼 라벨, `x-group` = 묶음 이름, `x-group-toggle: true` = 그 묶음 제목 자리의 스위치):
- 묶음 "크기": `resize: bool = false` (/// 크기 바꾸기, 스위치)
  - `size: SizeMode = LongEdge` (/// 가로 기준 Width, /// 세로 기준 Height, /// 긴 변 기준 LongEdge, /// 정확한 크기 Exact, /// SNS 규격 Preset; 활성: resize)
  - `width: u32 = 1920` (min 1, 활성: resize, size ∈ width·longEdge·exact), `height: u32 = 1080` (min 1, 활성: resize, size ∈ height·exact)
  - `preset: Preset = Og` (/// OG 1200×630 Og, /// X(트위터) 1600×900 X, /// 인스타 정사각 1080×1080 InstaSquare, /// 인스타 세로 1080×1350 InstaPortrait, /// 스토리 1080×1920 Story; 활성: resize, size = preset)
  - `fit: Fit = Cover` (/// 비율 유지(안에 맞춤) Contain, /// 늘리기 Stretch, /// 크롭해서 채우기 Cover, /// 여백 추가 Pad; 활성: resize, size ∈ exact·preset)
  - `noUpscale: bool = true` (활성: resize)
- 묶음 "포맷": `convert: bool = false` (/// 포맷 바꾸기, 스위치)
  - `format: OutFmt = Webp` (/// JPG Jpg, /// PNG Png, /// WebP Webp; 활성: convert)
  - `lossless: bool = false` (/// 무손실 (WebP); 활성: convert, format = webp)
- `background: String = "#ffffff"` (/// 투명 부분 배경색, format color; 활성: [convert, format = jpg] **또는** [resize, fit = pad] — 실제 출력이 JPG일 때도 쓰므로 묶음 밖)
- 묶음 "용량": `compress: bool = true` (/// 용량 줄이기, 스위치)
  - `mode: QualityMode = Quality` (/// 품질 지정 Quality, /// 목표 용량 TargetKb; 활성: compress)
  - `quality: u8 = 80` (1~100 슬라이더; 활성: compress, mode = quality)
  - `targetKb: u32 = 200` (min 1; 활성: compress, mode = targetKb)
  - `pngColors: u16 = 0` (/// PNG 색상 수 (0은 줄이지 않음), 0~256; 활성: compress)
- `metadata: MetaMode` (묶음 "저장")
- 스키마 루트에 `"x-require-any": ["resize", "convert", "compress"]` — 폼이 하나도 안 켜졌으면 실행·대상 확인을 막는다(Task 4). 도구도 `InvalidOptions("적용할 단계를 하나 이상 켜세요.")`.
- 크기 계산은 옛 `target_size`를 옮겨 Preset·Pad를 더한다: Preset은 그 크기의 Exact와 같다. Pad = Contain으로 줄인 뒤 상자 크기 캔버스(배경색, PNG·WebP 출력은 투명) 가운데에 놓기.
- 출력 포맷: `convert`면 `format`, 아니면 입력 포맷(GIF·BMP·TIFF 입력은 PNG로 저장 — 압축할 수 없는 포맷). JPG는 알파를 배경색으로 합성.
- 인코딩: `compress`면 JPG·WebP는 mode에 따라 품질/목표 용량(옛 `search_quality`), PNG는 `pngColors` 적용 후 oxipng preset 4. `compress`가 꺼져 있으면 JPG·WebP 품질 95, PNG는 oxipng 무손실(preset 2). WebP `lossless`면 품질·목표 용량 무시.
- meta: id `image.optimize`, name `이미지 최적화`, description `크기·포맷·품질을 한 번에 맞춥니다.`, extensions `INPUT_EXTS`

- [ ] **Step 1: 테스트 옮기기 + 새 테스트** — 옛 4개 크레이트의 테스트를 새 옵션 이름으로 옮긴다(target_size 표, no_upscale, huge cap, cover, png→jpg 배경, webp 무손실, keep metadata 방향, search_quality 3종, png 무손실·색상 수·투명 유지, schema slider). 추가: `resize_off_ignores_width`(resize 꺼짐 + width 10 → 크기 그대로), `default_only_compresses`(옵션 없이 JPG → JPG, 크기 같음, 용량 줄어듦), `resize_only_keeps_quality_high`(resize만 켜고 JPG → 품질 95로 저장: 같은 이미지 품질 95 인코딩과 크기 ±1%), `convert_only_png_to_webp`(convert만 → `.webp`, 크기 같음), `nothing_on_is_invalid`, `preset_og_is_1200x630`, `pad_fills_box_with_background`(400×300 → Exact 300×300 Pad, JPG 출력 → 300×300, 위·아래 띠가 배경색), `lossless_ignores_quality`, `gif_input_saves_png`, `schema_has_group_toggles`(resize·convert·compress에 `x-group-toggle: true`, 루트 `x-require-any`)
- [ ] **Step 2:** FAIL → **Step 3:** 구현, 옛 4개 크레이트 삭제 → **Step 4:** PASS + clippy
- [ ] **Step 5: Commit** — `refactor: 이미지 최적화 도구로 리사이즈·포맷 변환·압축 합침 (+SNS 규격, 여백 추가)`

### Task 2: `image.edit` (회전 → 크롭 → 여백)

**Files:** Create `crates/tools/image-edit/`(옛 `image-rotate`의 `build.rs`·`src/lt_transform.c`·FFI를 그대로 옮김); 옛 `image-rotate`, `image-crop`, `image-strip-meta` 삭제

**Interfaces — 옵션 `EditOptions`:**
- `angle: Angle = None`, `flipH`, `flipV` (옛 rotate와 같음, 기본 회전 없음)
- `crop: CropMode = None` (/// 자르지 않음 None, /// 비율 Ratio, /// 픽셀 Pixels), `ratio`, `width`, `height`(활성: crop = pixels), `anchor`(활성: crop ≠ none), `ratio` 활성: crop = ratio
- `margin: MarginMode = None` (/// 그대로 None, /// 여백 자동 자르기 Trim, /// 여백 추가 Pad, /// 정사각형 캔버스 Square)
- `tolerance: u8 = 10` (0~100 슬라이더, 활성: margin = trim), `padding: u32 = 20` (min 0, 활성: margin = pad)
- `transparent: bool = true` (/// 여백을 투명하게 (PNG·WebP), 활성: margin ∈ pad·square), `background: String = "#ffffff"` (color, 활성: margin ∈ pad·square)
- `quality: u8 = 95`, `metadata: MetaMode`
- 순서: 회전·뒤집기 → 크롭(옛 `crop_rect`) → 여백. 모두 "없음"이면 `InvalidOptions("회전·크롭·여백 중 하나 이상 고르세요.")`.
- JPG 무손실 경로는 크롭·여백이 없고 회전·뒤집기만 있을 때(옛 조건 그대로).
- 트림: 투명 픽셀이 있으면 알파 ≤ tolerance×2.55인 바깥 테두리를, 없으면 왼쪽 위 픽셀 색과 채널 차이 ≤ tolerance×2.55인 테두리를 잘라낸다(전부 지워지면 원본 유지).
- 여백·정사각형: 투명 지원 포맷(PNG·WebP·GIF)이고 `transparent`면 알파 0, 아니면 배경색.
- meta: id `image.edit`, name `자르기 · 회전 · 여백`, description `회전·뒤집기, 크롭, 여백 자르기·추가를 합니다.`

- [ ] **Step 1: 테스트 옮기기 + 새 테스트** — 옛 rotate 6개(무손실 왕복 포함)·crop 2개를 옮기고, 추가: `trim_removes_transparent_border`(투명 테두리 10px인 50×50 PNG → 30×30), `trim_removes_solid_border`(흰 테두리 JPG → 안쪽 크기, ±2px), `pad_adds_pixels`(+20 → 각 변 40 증가), `square_canvas_centers`(400×300 → 400×400, 위 50px 투명), `rotate_then_crop_order`(40×20 → 90° → 1:1 크롭 → 20×20), `nothing_selected_is_invalid`
- [ ] **Step 2:** FAIL → **Step 3:** 구현, 옛 3개 크레이트 삭제 → **Step 4:** PASS + clippy
- [ ] **Step 5: Commit** — `refactor: 자르기·회전·여백 도구로 회전·크롭 합침 (+여백 자르기·추가·정사각형)`

### Task 3: 카탈로그·내장 목록 정리

**Files:** `catalog/tools.json`, `crates/builtin/{Cargo.toml,src/lib.rs}`

- 이미지 항목을 plan.md 4장 8종으로 교체: `image.optimize`·`image.edit` (builtin true), `image.watermark`·`image.svg`·`image.gif_optimize`·`image.sprite`·`image.base64`·`image.icon` (false). id·이름·설명은 plan.md 4장과 같게. PDF·웹·기타 항목은 아직 없음(이후 Phase).
- `builtin()`: `dev-copy`, `image-optimize`, `image-edit`

- [ ] **Step 1:** 테스트 `catalog_has_eight_image_tools`로 바꿔 FAIL 확인 → **Step 2:** 교체 → **Step 3:** `cargo test --workspace` PASS
- [ ] **Step 4: Commit** — `chore: 카탈로그를 이미지 도구 8종으로 정리`

### Task 4: 옵션 연결 (`x-enabled-when`)

**Interfaces:**
- 스키마: `"x-enabled-when": { "<필드>": [허용 값…], … }`(모든 조건 AND) 또는 그 객체들의 배열(하나라도 맞으면 활성, OR). Rust: `#[schemars(extend("x-enabled-when" = { "format": ["jpg", "webp"], "lossless": [false] }))]`, OR은 `= [{ "format": ["jpg"] }, { "fit": ["pad"] }]`.
- 각 필드의 조건은 Task 1·2 옵션 목록의 "활성:" 그대로.
- TS: `Field`에 `enabledWhen?: Record<string, unknown[]>[]`(객체 하나도 배열 하나로 정규화), `isEnabled(field, values): boolean`, `OptionField`는 비활성이면 흐리게(`opacity-50`) + 조작 불가.
- 폼 묶음: 필드 순서대로 그리고, `"x-group": "크기"`가 바뀌는 곳에 묶음 제목(`text-2xs font-medium text-muted-foreground`)을 붙인다. `"x-group-toggle": true`인 체크박스 필드는 묶음 제목 자리에 "☑ 크기 바꾸기"처럼 그리고, 꺼져 있으면 그 묶음의 다른 칸은 (각자의 `x-enabled-when`에 따라) 흐려진다. `image.optimize`는 크기·포맷·용량·저장, `image.edit`는 회전·크롭·여백·저장.
- 필수 조건: 스키마 루트 `"x-require-any": [필드…]` → TS `requireAnyMissing(schema, values): string | null`(하나도 true가 아니면 `"적용할 단계를 하나 이상 켜세요."`), 작업 화면은 `canRun`보다 먼저 이 문구를 실행 막는 이유로 쓴다.

- [ ] **Step 1: 실패하는 테스트** — Rust: optimize·edit 스키마의 `quality`/`background`/`tolerance`에 기대 조건; TS: `schema-to-fields`(enabledWhen 정규화, x-group·x-group-toggle 읽기), `is-enabled.test.ts`(조건 없음 true, AND 하나 틀림 false, OR 하나 맞음 true, 배열에 값 있으면 true), `require-any.test.ts`(모두 false → 문구, 하나 true → null, 키 없음 → null)
- [ ] **Step 2:** FAIL → **Step 3:** 구현 → **Step 4:** PASS
- [ ] **Step 5: Commit** — `feat: 옵션 연결과 묶음 — 다른 옵션 값에 따라 칸 비활성`

### Task 5: Rust 미리보기

**Interfaces:**
- `localtools_image_common::DisplayImage { jpeg: Vec<u8>, width: u32, height: u32 }`(원본 크기), `display_image(path, max_side) -> Result<DisplayImage, ToolError>` — `load` → 긴 변 > max_side면 `thumbnail` → 알파는 체커 대신 흰색 합성 → JPEG 85
- `localtools_core::batch::preview_one(tool, input, opts) -> Result<(TempDir, Vec<PathBuf>), String>` — 새 임시 폴더에서 `tool.run`(catch_unwind → `"도구 실행 중 오류가 발생했습니다."`), 결과 경로 검사(엔진과 같은 규칙), 기록·백업 없음. `tempfile`을 core 일반 의존성으로.
- Tauri: `thumbnail(path) -> Result<String, String>`(`display_image(…, 160)` data URL), `preview_tool(toolId, input, opts) -> Result<ToolPreview, String>` — `ToolPreview { before: Shown, after: Option<Shown>, sizeBefore: u64, sizeAfter: Option<u64> }`, `Shown { url, width, height }`, 화면용 긴 변 1600, 크기는 실제 바이트, `spawn_blocking`

- [ ] **Step 1: 실패하는 테스트** — `display_image_is_downscaled`(4000×3000 → 디코딩 긴 변 1600, width/height는 원본, < 1MB), `display_image_flattens_alpha`, core `preview_leaves_no_trace`, `preview_reports_tool_error`(실패·패닉)
- [ ] **Step 2:** FAIL → **Step 3:** 구현 → **Step 4:** PASS
- [ ] **Step 5: Commit** — 본문에 `base64 0.22 추가: 화면용 이미지 data URL (MIT/Apache-2.0)`

### Task 6: 화면 — 카테고리 탭, 썸네일, 비교 화면, 두 열 배치

**Files:** Create `app/src/widgets/category-tabs/`, `app/src/features/image-preview/{api/preview-api.ts, api/use-thumbnail.ts, api/use-tool-preview.ts, lib/clamp-percent.ts(+test), ui/compare-slider.tsx, ui/compare-side-by-side.tsx, ui/preview-pane.tsx}`; Modify `features/tool-search/lib/group-tools.ts`(탭 안 목록: 즐겨찾기 + 나머지, 카테고리 제목 없음), `features/file-input/ui/input-list.tsx`(썸네일·선택), `features/job-run/ui/run-bar.tsx`("대상 확인"), `features/tool-options/ui/*`(비활성·그룹), `widgets/app-sidebar`, `widgets/tool-workspace/ui/file-tool-workspace.tsx`, `pages/home/ui/home-page.tsx`, `entities/tool/model/types.ts`

**Interfaces:**
- `availableCategories(tools: ToolEntry[]): string[]` — `CATEGORY_ORDER` 순, 도구가 하나 이상인 카테고리만
- `CategoryTabs({ categories, value, onChange, locked, className })` — 창 맨 위 `h-9 border-b bg-titlebar` 탭 막대, 선택 탭 `bg-background` + 아래 테두리 없음, 실행 중이면 잠금(사이드바와 같은 `locked`)
- `HomePage`: `tab` 상태(기본 = 선택 도구의 카테고리, 처음엔 `image`), 사이드바에는 그 탭 도구만
- `groupTools(tools, favorites)` → `[{ key: "favorites", label: "즐겨찾기" }, { key: "all", label: "도구" }]`(빈 그룹 생략) — 탭이 카테고리를 대신
- `useThumbnail(path, enabled)`, `useToolPreview(toolId, path, opts)`(300ms 디바운스, `keepPreviousData`, `retry: false`)
- `clampPercent(x, width)` — 0~100(폭 0이면 50)
- `CompareSlider({ before, after })` — 겹쳐 놓고 결과를 `clip-path: inset(0 0 0 X%)`, 막대 포인터·키보드(←/→ 5%) 이동, `role="slider"`
- `CompareSideBySide({ before, after })` — 두 칸 나란히(`grid grid-cols-2 gap-2`), 각 칸 라벨
- `PreviewPane({ toolId, path, opts })` — 위에 보기 전환(`슬라이더 | 나란히`, 선택은 `settings`가 아니라 화면 상태), 상태(없음/만드는 중/오류/결과), 아래 `formatChange` + `W×H → W×H`
- `InputList`: 32px 썸네일(폴더는 `IconFolder`), `selected`·`onSelect`, 선택 행 `bg-selection`
- `FileToolWorkspace`: `grid grid-cols-2 gap-6`, 오른쪽 `sticky top-0` `PreviewPane`, 선택 기본값 = 첫 파일

- [ ] **Step 1: 실패하는 테스트** — `clamp-percent.test.ts`, `available-categories.test.ts`(순서·빈 카테고리 제외), `group-tools.test.ts` 갱신(즐겨찾기·도구 두 그룹)
- [ ] **Step 2:** FAIL → **Step 3:** 구현 → **Step 4:** PASS, tsc, prettier, 금지 색 grep
- [ ] **Step 5: 수동 확인** — `npm run tauri dev`: 탭 전환, JPG 폴더 드롭 → 썸네일 → 첫 파일 비교 → 품질·크기를 바꾸면 결과·용량이 바뀜 → 무손실이면 품질 흐림 → 슬라이더/나란히 전환 → 실행·되돌리기; 브라우저 가짜 IPC로 라이트/다크 스크린샷
- [ ] **Step 6: Commit** — `feat: 카테고리 탭, 이미지 썸네일, 원본·결과 비교 미리보기`
