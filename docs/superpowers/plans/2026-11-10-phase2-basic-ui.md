# Phase 2 기본 UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** plan.md 7장의 파일 도구 화면 전체(드롭·입력 목록·스키마 옵션 폼·출력 정책·미리보기·실행·결과 리포트·되돌리기·폴더 열기), 카탈로그 기반 사이드바(검색·즐겨찾기·"설치 안 됨"), 설정 화면을 만든다.

**Architecture:** Rust는 카탈로그(`catalog/tools.json`)와 내장 도구를 합쳐 상태가 붙은 목록을 주고, 설정을 `%LOCALAPPDATA%\LocalTools\settings.json`에 읽고 쓰며, 시작할 때 오래된 작업 기록을 지운다. 프론트는 FSD로 화면을 조립하고, 판단이 들어가는 로직(스키마→폼 필드, 검색, 결과 요약, 테마 결정)은 순수 함수로 빼서 Vitest로 테스트한다. 화면 모양은 `docs/UI-SYSTEM.md` 4장 수치를 따른다.

**Tech Stack:** Rust (serde, schemars), Tauri 2 + `tauri-plugin-opener`, React 19, TanStack Query, Vitest(node), shadcn base-nova(`select`, `slider` 추가)

**Spec:** [docs/plan.md](../../plan.md) 7장·8장 Phase 2, [설계 문서](../specs/2026-11-10-image-first-addon-design.md) 4·5장, [docs/UI-SYSTEM.md](../../UI-SYSTEM.md)

## Global Constraints

- 화면 문구 한국어. 모달 닫기는 하단 취소 버튼(✕ 없음). 컴포넌트 props는 `type`, 파일 하나 = 컴포넌트 하나, 재사용 컴포넌트는 `className` prop.
- 화면 코드에 토큰 밖 색 금지(`#hex`, `bg-[…]`, `text-gray-500` 등). 움직임은 페이드만(`zoom-*`, `slide-in-from-*`, `translate-y-px` 금지).
- 기본 요소는 `@/shared/ui`를 거친다. 새 부품은 `npx shadcn@latest add <name>` 후 UI-SYSTEM.md 3장 4)절대로 손본다.
- FSD: `app → pages → widgets → features → entities → shared`, 다른 슬라이스는 `index.ts`로만.
- 테스트: Vitest, `environment: "node"`, 순수 함수만, 대상 옆 `*.test.ts`, 스냅샷 금지. 컴포넌트·훅은 테스트하지 않는다 (`~/dotfiles/claude/docs/nextjs-testing-setup.md`).
- 프론트는 네트워크를 쓰지 않는다. Tauri capability 추가는 `opener:allow-reveal-item-in-dir` 하나만.
- 의존성 추가 커밋에 이유·라이선스.
- 끝날 때마다 `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `npx tsc --noEmit`, `npx vitest run`, `npx prettier --check src`.

## 이번 계획에서 정한 것

- **화면 전환:** 라우터 없이 앱 상태 하나(`view: { type: "tool", id } | { type: "settings" }`). 데스크톱 창 하나라 주소창·뒤로 가기가 없다. 설정은 사이드바 맨 아래 고정 항목(UI-SYSTEM.md 4장 "맨 아래 고정 항목").
- **설정 저장:** 모든 설정(테마·동시 작업 수·기본 출력 정책·로그 보관 기간·즐겨찾기·도구별 마지막 옵션)을 `settings.json` 한 파일에. localStorage는 쓰지 않는다.
- **"기본 메타데이터 처리" 설정:** 메타데이터를 다루는 도구가 Phase 3에 생기므로 그때 추가.
- **Interactive 결과 뷰어:** Interactive 도구가 Phase 5(`base64`, `compare`)에 처음 생기므로 뷰어도 그때 만든다. 이번에는 Interactive 도구를 고르면 "준비 중" 문구.
- **`dev.copy`:** debug 빌드에서만 목록에 보인다(카테고리 `dev`).
- **Phase 0 UI 확인 패널(`widgets/ui-check`)** 삭제.

## 화면 구성

```
┌─────────────┬──────────────────────────────────────────────┐
│ [검색 h-7]   │ 도구 이름 (text-base)  설명 (muted)            │
│ ★ 즐겨찾기   ├──────────────────────────────────────────────┤
│  WebP 변환   │ ┌ 드롭 영역 ─────────────────────────────┐   │
│ 이미지       │ │ 파일·폴더를 끌어 놓으세요  [파일 선택][폴더 선택] │
│  WebP 변환 ☆ │ └───────────────────────────────────────┘   │
│  AVIF 변환   │ 입력 목록 표 (이름 · 크기 · 상태)  [모두 지우기] │
│  설치 안 됨  │ ── 옵션 ─────────────── ── 저장 방식 ───────── │
│  …          │ 스키마 폼                 (●) 원본 자리 + 백업  │
│ dev (debug) │                          ( ) 원본 옆 _접미사    │
│             │                          ( ) 다른 폴더 [선택]   │
│             │                          [✓] 하위 폴더 포함     │
│─────────────│ [미리보기] [실행] [취소]  ━━━━━━━ 3/10          │
│ ⚙ 설정       │ 결과: 성공 8 · 실패 1 · 건너뜀 1               │
│             │ 45.2MB → 12.1MB (-73%) [폴더 열기][되돌리기]   │
└─────────────┴──────────────────────────────────────────────┘
```

## Review Focus

1. `settings.json`이 없거나 깨졌을 때 → 기본값으로 열리고, 저장하면 올바른 파일로 덮어써진다. → Task 2 `load_settings_falls_back_on_corrupt_file`
2. 예전 버전에서 저장된 "마지막 옵션"에 없어진 키·범위 밖 값이 있을 때 → 아는 키만 쓰고 범위로 자르며 나머지는 기본값. → Task 4 `mergeOptions` 테스트
3. 폼이 그릴 수 없는 스키마 타입(배열·객체) → 화면이 깨지지 않고 해당 칸은 "이 옵션은 기본값을 사용합니다". → Task 4 `schemaToFields` 테스트
4. "다른 폴더" 정책인데 폴더를 고르지 않음 → 실행·미리보기 버튼 비활성 + 안내. → Task 6 `canRun` 테스트
5. 실행 중 파일을 또 드롭 → 무시(입력 목록이 바뀌지 않음). → Task 5 (드롭 핸들러가 `running`이면 반환) + 수동 확인

---

## 파일 구조

```
catalog/tools.json                       # 전체 도구 목록 (설계 6.5)
crates/builtin/src/lib.rs                # + catalog(), CatalogEntry
crates/builtin/src/settings.rs           # Settings, load_settings, save_settings, settings_path
crates/core/src/batch/prune.rs           # prune_logs
app/src-tauri/src/commands.rs            # list_tools(카탈로그 병합), stat_paths, get/set_settings
app/src-tauri/src/lib.rs                 # opener 플러그인, 시작 시 prune_logs
app/vitest.config.ts
app/src/shared/ui/{select,slider}.tsx    # shadcn add 후 손봄
app/src/shared/ui/radio-group.tsx        # shadcn add 후 손봄 (저장 방식 선택)
app/src/shared/lib/format-bytes.ts (+test)
app/src/shared/lib/resolve-theme.ts (+test)
app/src/entities/tool/        model/types.ts, config/category-labels.ts, api/use-tools.ts, ui/tool-row.tsx
app/src/entities/settings/    model/types.ts, api/{settings-keys,use-settings,use-save-settings}.ts, lib/apply-theme.ts
app/src/features/tool-search/ lib/filter-tools.ts(+test), ui/tool-search-input.tsx
app/src/features/tool-favorite/ ui/favorite-toggle.tsx
app/src/features/file-input/  api/stat-paths.ts, model/use-file-drop.ts, ui/{drop-zone,input-list}.tsx
app/src/features/tool-options/ lib/{schema-to-fields,merge-options}.ts(+test), ui/{options-form,option-field}.tsx
app/src/features/job-run/     api/job-api.ts, lib/{can-run,summarize-report}.ts(+test), model/use-job.ts,
                              ui/{policy-picker,preview-panel,run-bar,result-report}.tsx
app/src/widgets/app-sidebar/  ui/app-sidebar.tsx
app/src/widgets/tool-workspace/ ui/{tool-workspace,not-installed-panel}.tsx
app/src/pages/home/ui/home-page.tsx      # 사이드바 + 작업 영역/설정 전환
app/src/pages/settings/ui/settings-page.tsx
```

---

### Task 1: 카탈로그와 상태 붙은 도구 목록

**Files:** Create `catalog/tools.json`; Modify `crates/builtin/src/lib.rs`, `app/src-tauri/src/commands.rs`

**Interfaces — Produces:**
- `catalog/tools.json`: 배열. 항목 `{ "id", "name", "category", "description", "builtin": bool }`. 내용: 설계 4장 이미지 20종(id·이름은 plan.md 4장 표 그대로, `builtin`은 기본 내장 7종만 true — 단 아직 구현이 없으므로 이번 Phase에는 **모두 false**, Phase 3에서 구현하며 true로) + `dev.copy`(`category: "dev"`, `builtin: true`).
- `localtools_builtin::CatalogEntry { id, name, category, description, builtin: bool }` (`Deserialize`), `pub fn catalog() -> Vec<CatalogEntry>` — `include_str!("../../../catalog/tools.json")`. release 빌드에서는 `category == "dev"` 항목 제외.
- 명령 `list_tools() -> Vec<ToolEntry>`: `ToolEntry { id, name, category, description, status: "builtin" | "notInstalled", kind: Option<ToolKind>, options_schema: Option<Value>, extensions: Vec<String> }` camelCase. `builtin()`에 있는 도구는 `status: "builtin"` + kind·schema·extensions, 나머지는 `notInstalled` + `None`.

- [ ] **Step 1: 실패하는 테스트** (`crates/builtin/src/lib.rs`)
  - `catalog_matches_builtin`: `builtin()`의 모든 도구 id가 카탈로그에 `builtin: true`로 있고 이름이 같다; 카탈로그의 `builtin: true` 항목은 모두 `find(id)`로 찾힌다
  - `catalog_ids_are_unique_and_well_formed`: id 중복 없음, 모두 `카테고리.이름` 형식이고 앞부분이 `category`와 같다
  - `catalog_has_twenty_image_tools`: `category == "image"` 20개
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + clippy
- [ ] **Step 5: Commit** — `feat: 도구 카탈로그와 설치 상태가 붙은 도구 목록`

### Task 2: 설정 파일과 작업 기록 정리

**Files:** Create `crates/builtin/src/settings.rs`, `crates/core/src/batch/prune.rs`; Modify `app/src-tauri/src/{commands,lib}.rs`

**Interfaces — Produces:**
- `localtools_builtin::Settings` (`Serialize + Deserialize`, camelCase, 모든 필드 `#[serde(default)]`):
  `theme: Theme("system"|"light"|"dark", 기본 system)`, `threads: usize(0=자동)`, `default_policy: OutputPolicy(기본 InPlaceWithBackup)`, `keep_if_larger: bool(true)`, `log_retention_days: u32(30)`, `favorites: Vec<String>`, `last_options: BTreeMap<String, Value>`
- `pub fn settings_path() -> PathBuf` (`%LOCALAPPDATA%\LocalTools\settings.json`), `pub fn load_settings(path) -> Settings` (없거나 깨졌으면 `Default`, 깨졌으면 `eprintln!`), `pub fn save_settings(path, &Settings) -> io::Result<()>` (임시 파일에 쓰고 rename — 쓰다 꺼져도 기존 파일이 반쯤 남지 않게. 여기서는 자기 파일을 바꾸는 것이라 덮어쓰기 rename이 맞다)
- `localtools_core::batch::prune_logs(log_dir: &Path, older_than: Duration) -> usize` — 수정 시각이 기준보다 오래된 `*.jsonl` 삭제, 지운 수 반환. 폴더가 없으면 0.
- 명령: `get_settings() -> Settings`, `set_settings(settings: Settings) -> Result<(), String>`, `stat_paths(paths: Vec<PathBuf>) -> Vec<PathStat>` (`PathStat { path, size: u64, is_dir: bool, exists: bool }`, 폴더 size 0)
- `lib.rs` setup: 시작 시 `prune_logs(default_log_dir(), days * 86400초)` (설정의 `log_retention_days`, 0이면 정리 안 함)

- [ ] **Step 1: 실패하는 테스트**
  - `load_settings_defaults_when_missing`: 없는 경로 → `Settings::default()`와 같음 (`theme: system`, `log_retention_days: 30`, `keep_if_larger: true`)
  - `load_settings_falls_back_on_corrupt_file`: `"{not json"` → 기본값 (Review Focus 1)
  - `load_settings_fills_missing_fields`: `{"theme":"dark"}` → theme dark, 나머지 기본값
  - `save_then_load_roundtrip`: favorites·last_options 포함 저장 후 읽으면 같음
  - `prune_logs_removes_only_old_files` (core): 파일 2개 중 하나의 수정 시각을 `File::set_modified`로 40일 전으로 → `prune_logs(dir, 30일)` == 1, 새 파일은 남음
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + clippy
- [ ] **Step 5: Commit** — `feat: 설정 파일 저장과 오래된 작업 기록 정리`

### Task 3: Vitest 도입 + 공용 순수 함수 + UI 부품 추가

**Files:** Create `app/vitest.config.ts`, `app/src/shared/lib/{format-bytes,resolve-theme}.ts(+test)`, `app/src/shared/ui/{select,slider,radio-group}.tsx`; Copy `D:\project\git\udrive\src\components\ui\desktop-style.test.ts` → `app/src/shared/ui/` (경로 `../../index.css` → `../../app/styles/index.css`); Modify `app/package.json`(script `"test": "vitest run"`), `app/.prettierignore`(새 shadcn 파일은 포맷 대상 — 원본 복사본이 아님)

**Interfaces — Produces:**
- `formatBytes(n: number): string` — 1024 단위, `B`/`KB`/`MB`/`GB`, 소수 한 자리(B는 정수): `0 → "0 B"`, `1536 → "1.5 KB"`, `47396249 → "45.2 MB"`
- `formatChange(before: number, after: number): string` — `"45.2 MB → 12.1 MB (-73%)"`, before 0이면 `"0 B → … "` 퍼센트 생략, 증가는 `+`
- `resolveTheme(theme: "system" | "light" | "dark", prefersDark: boolean): "light" | "dark"`
- `@/shared/ui/select` (`Select*`), `@/shared/ui/slider` (`Slider`), `@/shared/ui/radio-group` (`RadioGroup`, `RadioGroupItem`) — shadcn base-nova로 받고 `zoom-*`/`slide-in-from-*`/`translate-y-px` 제거, select 팝업은 `menu-styles`의 `menuPopup`/`menuItem` 사용, 높이 26px(`h-6.5`)

- [ ] **Step 1:** `npm i -D vitest` + `vitest.config.ts`(`environment: "node"`, `include: ["src/**/*.test.ts"]`, `@` 별칭) — 가이드 템플릿 기준
- [ ] **Step 2: 실패하는 테스트** — `format-bytes.test.ts`(위 3값 + `formatChange(47396249, 12687769) === "45.2 MB → 12.1 MB (-73%)"`, `formatChange(100, 150) === "100 B → 150 B (+50%)"`), `resolve-theme.test.ts`(system+dark→dark, system+light→light, light+dark→light)
- [ ] **Step 3:** `npx vitest run` → FAIL 확인 → **Step 4:** 구현 → PASS
- [ ] **Step 5:** shadcn add 3종 → 손보기 → `desktop-style.test.ts`가 그대로 통과(+ `ANIMATED` 목록에 `select.tsx` 추가), `tsc` 통과
- [ ] **Step 6: Commit** — 본문에 `vitest 추가: 순수 함수 단위 테스트 (MIT)`

### Task 4: 옵션 폼 (스키마 → 필드)

**Files:** `app/src/features/tool-options/{index.ts, lib/schema-to-fields.ts(+test), lib/merge-options.ts(+test), ui/options-form.tsx, ui/option-field.tsx}`

**Interfaces — Produces:**
- `type Field =`
  - `{ kind: "checkbox"; key; label; description?; default: boolean }` — `type: "boolean"`
  - `{ kind: "slider"; key; label; description?; min; max; step; default: number }` — `integer|number`이고 `minimum`·`maximum` 둘 다 있음 (`step`: integer면 1, 아니면 0.1)
  - `{ kind: "number"; key; label; description?; min?; max?; integer: boolean; default: number | null }` — 범위가 하나라도 없음
  - `{ kind: "select"; key; label; description?; options: { value: string; label: string }[]; default: string }` — `enum` 또는 `oneOf[].const`(+`description`을 label로), `$ref`면 `$defs`에서 한 단계 해석
  - `{ kind: "color"; key; label; description?; default: string }` — `type: "string"`, `format: "color"`
  - `{ kind: "text"; key; label; description?; default: string }` — 그 밖의 `string`
  - `{ kind: "unsupported"; key; label }` — 배열·객체 등 (Review Focus 3)
- `schemaToFields(schema: unknown): Field[]` — `properties` 순서대로. `label`은 `title` ?? `description` ?? key, `description`은 title이 있을 때만. 잘못된 스키마(객체 아님 등) → `[]`.
- `mergeOptions(fields: Field[], saved: unknown): Record<string, unknown>` — 각 필드의 기본값에서 시작해 `saved`에 같은 키·같은 타입이 있으면 덮어씀; 숫자는 min/max로 자르고 integer면 반올림, select는 options에 없는 값이면 기본값; 모르는 키는 버림 (Review Focus 2)
- `OptionsForm({ schema, value, onChange, disabled, className })` — `value`는 `mergeOptions` 결과. 필드별 UI: checkbox → `Checkbox`, slider → `Slider` + 오른쪽 숫자, number → `Input type=number`, select → `Select`, color → `Input type=color`, text → `Input`, unsupported → `text-muted-foreground` "이 옵션은 기본값을 사용합니다". 필드가 없으면 "설정할 옵션이 없습니다."

- [ ] **Step 1: 실패하는 테스트** — schemars가 실제로 내는 형태로 픽스처 작성:
  - `schema-to-fields.test.ts`: 품질 `{type:"integer",minimum:0,maximum:100,default:80,title:"품질"}` → slider(step 1); `{type:"boolean",default:false,title:"무손실"}` → checkbox; `{$ref:"#/$defs/Mode"}` + `$defs.Mode = {oneOf:[{const:"quality",description:"품질 지정"},{const:"size",description:"목표 KB"}]}`, `default:"quality"` → select 옵션 2개 라벨 한국어; `{type:"array"}` → unsupported; `{type:"integer",minimum:1}` → number; `schemaToFields(null)` → `[]`; `{type:"string",format:"color",default:"#ffffff"}` → color
  - `merge-options.test.ts`: saved `{quality: 150, gone: 1, lossless: "yes"}` → `{quality: 100, lossless: false}`(범위로 자름·모르는 키 버림·타입 틀리면 기본값); saved `{mode:"zzz"}` → mode 기본값; saved `null` → 모두 기본값; integer 필드 `42.6` → `43`
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS, tsc
- [ ] **Step 5: Commit** — `feat: 스키마 기반 옵션 폼`

### Task 5: 입력 받기 (드롭·선택·목록)

**Files:** `app/src/features/file-input/{index.ts, api/stat-paths.ts, model/use-file-drop.ts, ui/drop-zone.tsx, ui/input-list.tsx}`

**Interfaces — Produces:**
- `statPaths(paths: string[]): Promise<PathStat[]>` (`invoke("stat_paths")`), `PathStat = { path; size; isDir; exists }` — `entities/tool/model/types.ts`에 타입 추가
- `useFileDrop(onDrop: (paths: string[]) => void, enabled: boolean): { over: boolean }` — `getCurrentWebview().onDragDropEvent`: `enter`/`over` → over true, `leave`/`drop` → false, `drop`이고 `enabled`면 `onDrop(paths)`. 언마운트 시 unlisten. (Review Focus 5: `enabled = !running`)
- `DropZone({ onAdd, disabled, className })` — 점선 테두리 영역(`rounded-md border border-dashed`, over면 `border-primary bg-primary/10`), 문구 "파일이나 폴더를 끌어 놓으세요", 버튼 "파일 선택"(`open({ multiple: true })`) · "폴더 선택"(`open({ directory: true, multiple: true })`)
- `InputList({ items: PathStat[], results?: Map<string, FileResult>, onRemove, onClear, className })` — UI-SYSTEM.md 4장 목록 보기 표: 열 이름 · 크기(`formatBytes`, 폴더는 "폴더") · 상태(결과 없으면 "대기", `done` "완료", `skipped` "건너뜀(사유)", `failed` 빨강 메시지, `cancelled` "취소됨"), 행 `h-8.5`, 줄무늬, 오른쪽 끝 제거 버튼(`icon-xs` ghost, `IconX` — 모달 닫기가 아니라 행 삭제라 규칙과 무관), 위에 "모두 지우기"
- 입력 상태(목록 · 추가 · 중복 제거 · 제거)는 `widgets/tool-workspace`가 `useState`로 갖는다 (중복은 경로 문자열 기준)

- [ ] **Step 1:** 구현 (컴포넌트·훅이라 테스트 없음 — 가이드 비대상)
- [ ] **Step 2:** tsc, prettier, 금지 색 grep 0건
- [ ] **Step 3: Commit** — `feat: 파일·폴더 드롭과 입력 목록`

### Task 6: 실행 흐름 (정책·미리보기·실행·결과)

**Files:** `app/src/features/job-run/{index.ts, api/job-api.ts, lib/can-run.ts(+test), lib/summarize-report.ts(+test), model/use-job.ts, ui/policy-picker.tsx, ui/preview-panel.tsx, ui/run-bar.tsx, ui/result-report.tsx}`; Delete `app/src/features/tool-run/`; Modify `app/src-tauri/capabilities/default.json`(+`opener:allow-reveal-item-in-dir`), `app/src-tauri/Cargo.toml`(+`tauri-plugin-opener = "2"`), `lib.rs`(`.plugin(tauri_plugin_opener::init())`), `app/package.json`(+`@tauri-apps/plugin-opener`)

**Interfaces — Produces:**
- `job-api.ts`: Phase 1 `features/tool-run/api/run-job.ts` 함수들을 옮김 + `previewJob(toolId, spec): Promise<Plan>` (`Plan = { files: PlannedFile[]; skipped: Skipped[] }`, `PlannedFile = { input; root; rel; destDir }` 타입을 entities에 추가)
- `canRun(inputs: string[], policy: OutputPolicy, running: boolean): { ok: true } | { ok: false; reason: string }` — 입력 없음 → `"처리할 파일을 추가하세요."`, `OutputFolder`인데 `dir`가 빈 문자열 → `"저장할 폴더를 선택하세요."`, `KeepOriginal`인데 suffix 빈 문자열 → `"접미사를 입력하세요."`, 실행 중 → `"실행 중입니다."` (Review Focus 4)
- `summarizeReport(report: JobReport): { done; failed; skipped; cancelled; sizeBefore; sizeAfter; change: string | null; openDir: string | null }` — `skipped`는 파일 결과의 skipped + 계획 단계 `report.skipped` 합계, 용량은 `done`만 합산, `change`는 `done`이 없으면 null 아니면 `formatChange`, `openDir`는 첫 `done`의 첫 output의 부모 폴더 (없으면 null)
- `useJob(toolId)`: `{ preview, run, cancel, undo, progress, report, undoItems, error, running }` — Phase 1 `tool-runner.tsx` 로직(newJobId → runJob, 오류 배너용 문구)을 훅으로 옮김
- `PolicyPicker({ value, onChange, recursive, onRecursiveChange, disabled, className })` — `RadioGroup` 3개: "원본 자리에 저장 (원본은 original 폴더로)", "원본 옆에 저장" + 접미사 `Input`(기본 `_` + 도구 id의 마지막 부분, 예 `_webp`), "다른 폴더에 저장" + 폴더 경로 표시 + "선택" 버튼(`open({ directory: true })`); 아래 `Checkbox` "하위 폴더 포함"
- `PreviewPanel({ plan, className })` — "대상 n개 · 건너뜀 m개", 건너뜀 목록(경로: 사유), 결과 위치(첫 3개 `destDir`, 나머지 "외 k곳")
- `RunBar({ onPreview, onRun, onCancel, running, progress, disabledReason, className })` — 버튼 3개 + `Progress` + `done/total`; `disabledReason`이 있으면 버튼 옆에 `text-xs text-muted-foreground`로 표시
- `ResultReport({ report, undoItems, onUndo, undoing, className })` — "성공 n · 실패 n · 건너뜀 n · 취소 n", `change`, "폴더 열기"(`revealItemInDir(openDir)`), "되돌리기" → 끝나면 "되돌렸습니다 (처리 n, 건너뜀 m)" + 건너뛴 항목 목록(경로: 사유)
- 실행 spec: `{ inputs, recursive, policy, keepIfLarger: settings.keepIfLarger, threads: settings.threads, opts: 옵션 폼 값 }`. 실행할 때 옵션 값을 `settings.lastOptions[toolId]`에 저장.

- [ ] **Step 1: 실패하는 테스트** — `can-run.test.ts`(위 4개 사유 + 정상), `summarize-report.test.ts`(done 2·failed 1·파일 skipped 1·계획 skipped 2 → skipped 3, 용량 합, openDir = 첫 결과 폴더; done 0 → change null, openDir null; Windows 경로 `C:\a\b.png` → `C:\a`)
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS, tsc
- [ ] **Step 5: Commit** — 본문에 `tauri-plugin-opener 추가: 결과 폴더를 탐색기로 열기 (MIT/Apache-2.0)`

### Task 7: 사이드바·작업 영역·설정 화면 조립

**Files:** `app/src/entities/tool/*`(`ToolEntry` 타입, `config/category-labels.ts`, `ui/tool-row.tsx`; `ToolList` 삭제), `app/src/entities/settings/*`, `app/src/features/tool-search/*`, `app/src/features/tool-favorite/*`, `app/src/widgets/app-sidebar/*`, `app/src/widgets/tool-workspace/*`, `app/src/pages/home/ui/home-page.tsx`, `app/src/pages/settings/*`, `app/src/app/app.tsx`; Delete `app/src/widgets/ui-check/`

**Interfaces — Produces:**
- `ToolEntry` = Task 1 형태(TS), `CATEGORY_LABELS: Record<string, string>` = `{ image: "이미지", pdf: "PDF", web: "웹·폰트", misc: "기타", dev: "개발용" }`, 순서 `CATEGORY_ORDER = ["image","pdf","web","misc","dev"]`
- `filterTools(tools: ToolEntry[], query: string): ToolEntry[]` — 공백 제거·대소문자 무시, 이름·설명·id에 포함되면 남김; 빈 검색어면 그대로
- `groupTools(tools: ToolEntry[], favorites: string[]): { key: string; label: string; tools: ToolEntry[] }[]` — 맨 앞 `"favorites"`("즐겨찾기", 즐겨찾기 순서대로, 비면 그룹 생략), 그다음 `CATEGORY_ORDER` 순 카테고리(빈 그룹 생략)
- `Settings` TS 타입(Task 2 camelCase), `useSettings()`(`invoke("get_settings")`, `settingsKeys`), `useSaveSettings()`(낙관적 업데이트 + 실패 시 롤백 + `onSettled` invalidate)
- `applyTheme(theme)` — `resolveTheme(theme, matchMedia("(prefers-color-scheme: dark)").matches)`로 `<html>`의 `dark` 클래스 토글; `system`이면 media 변경도 구독. `app.tsx`에서 설정이 바뀔 때 호출.
- `ToolRow({ tool, selected, favorite, onSelect, onToggleFavorite, className })` — UI-SYSTEM.md 사이드바 행(`my-px flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium`, 선택 `bg-selection`), `notInstalled`면 이름 뒤 `text-2xs text-muted-foreground` "설치 안 됨", 오른쪽에 hover 때만 보이는 별(`IconStar`/`IconStarFilled`, `size-4`, `stroke={1.75}`)
- `AppSidebar({ tools, favorites, selectedId, view, onSelect, onOpenSettings, onToggleFavorite, className })` — 위 검색 `Input`(h-7, placeholder "도구 검색"), 그룹 제목 `text-2xs font-medium text-muted-foreground`, 맨 아래 고정 "설정"(`mt-auto border-t border-sidebar-border pt-2`)
- `ToolWorkspace({ tool, settings, className })` — `status === "notInstalled"` → `NotInstalledPanel`(이름·설명 + 비활성 버튼 "설치" + `text-xs text-muted-foreground` "추가 설치는 다음 업데이트에서 지원합니다."); `kind === "interactive"` → "준비 중" 문구; 그 밖엔 DropZone → InputList → (OptionsForm | PolicyPicker 2단) → RunBar → PreviewPanel/ResultReport. `key={tool.id}`로 도구가 바뀌면 상태 초기화. 정책 기본값은 `settings.defaultPolicy`.
- `SettingsPage({ className })` — 항목: 테마(`Select`: 시스템/라이트/다크), 동시 작업 수(`Select`: 자동, 1~16), 기본 저장 방식(`PolicyPicker`에서 폴더 선택은 제외한 단순 `Select` 3개 — "다른 폴더"는 작업마다 고르므로 기본값 후보에서 뺌: 원본 자리/원본 옆), 용량이 커지면 원본 유지(`Checkbox`), 작업 기록 보관 기간(`Select`: 7/30/90일/보관 안 함=0). 바뀌면 바로 저장(`useSaveSettings`), 저장 실패는 `ErrorBanner`.
- `HomePage` — `view` 상태, 처음엔 즐겨찾기 첫 도구 또는 첫 `builtin` 도구; 바깥 레이아웃은 UI-SYSTEM.md 4장 창 레이아웃(사이드바 `w-60`, 메인 `overflow-auto`)

- [ ] **Step 1: 실패하는 테스트** — `filter-tools.test.ts`(이름 부분 일치, 대소문자·공백 무시, id로 찾기, 빈 검색어), `group-tools.test.ts`(즐겨찾기 그룹이 맨 앞·즐겨찾기 순서 유지, 빈 카테고리 생략, 즐겨찾기 없으면 그룹 없음; `groupTools`는 `features/tool-search/lib/`에 둔다)
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS
- [ ] **Step 5:** 정적 확인 — tsc, `vite build`, prettier, 금지 색 grep 0건, `npx vitest run`(desktop-style 포함)
- [ ] **Step 6: 수동 확인** — `npm run tauri dev`:
  - 사이드바: 이미지 20종이 "설치 안 됨"으로 보이고 검색으로 걸러짐, 별로 즐겨찾기 → 맨 위 그룹, 앱을 다시 켜도 유지
  - "파일 복사(테스트)": 폴더 드롭 → 목록·크기 → 미리보기 → 실행(진행률) → 결과 리포트·용량 → 폴더 열기 → 되돌리기
  - "다른 폴더" 고르고 폴더 미선택 → 버튼 비활성 + 안내 (Review Focus 4)
  - 실행 중 드롭 → 무시 (Review Focus 5)
  - 설정: 테마 라이트/다크/시스템 즉시 반영, 다시 켜도 유지
  - 브라우저(`npx vite`) + playwright-cli로 라이트·다크 스크린샷 1장씩 (Tauri 밖이라 목록은 오류 배너가 정상)
- [ ] **Step 7: Commit** — `feat: 사이드바·작업 화면·설정 화면 (Phase 2)`

---

## Phase 2 완료 기준 대조 (plan.md 8장)

| 기준 | 확인 |
| --- | --- |
| 폴더 드롭 → 미리보기 → 실행 → 결과 → 되돌리기 (UI) | Task 7 Step 6 |
| 설치 안 된 도구 화면 표시 | Task 1 + Task 7 Step 6 |
| 스키마 기반 옵션 폼, 설정 화면 | Task 4, Task 7 |
