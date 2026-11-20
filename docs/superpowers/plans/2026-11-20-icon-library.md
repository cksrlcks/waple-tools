# 아이콘 라이브러리 (2판) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 아이콘 라이브러리를 추가 설치 도구(DLL)로 만들고, 앱에는 공용 부품(도구 리소스 받기, `resourceList`·`iconGrid` 블록, 스키마 플래그)만 둔다.

**Architecture:** DLL `crates/tools/web-icon-library`가 내장 세트 목록과 받은 Iconify 세트로 결과 블록(목록·격자·PNG·코드)을 만든다. 앱 설치 모듈의 공용 "도구 리소스"가 도구가 선언한 npm 패키지를 사용자가 누를 때만 받아 `addon-data/<tool-id>/`에 저장하고, 실행 때 `__resources`로 경로를 넘긴다. 프론트는 새 블록 두 개와 플래그(`x-hidden`, `x-step`, `x-stage`, `x-stage-plain`, `x-panel-when`, `x-panel-blocks`)만 안다.

**Tech Stack:** Rust (core `Block`, schemars, resvg 0.49·tiny-skia, `usvg::roxmltree`, tar/flate2/sha2 — 모두 Cargo.lock에 있음), Tauri 2, React 19 + TanStack Query + Vitest.

**Spec:** [docs/superpowers/specs/2026-11-20-icon-library-design.md](../specs/2026-11-20-icon-library-design.md) (2판)

## Global Constraints

- 아이콘·Iconify 개념은 앱 코드(app/src, app/src-tauri)에 넣지 않는다. 앱은 공용 부품만.
- 네트워크: 사용자가 [받기]·[업데이트]·[업데이트 확인]을 누를 때만. 받는 곳은 `registry.npmjs.org`(https)뿐. 앱 시작·화면 진입·도구 실행은 네트워크를 쓰지 않는다. DLL은 네트워크 크레이트를 쓰지 않는다.
- 리소스 id·아이콘 이름 규칙 `^[a-z0-9]+(-[a-z0-9]+)*$`. 버전은 `store::safe_name` 통과 시에만 폴더로.
- 무료 라이선스: spdx `MIT`, `Apache-2.0`, `ISC`, `CC0-1.0`, `hidden` 아님.
- 배치 작업 중 리소스 받기 금지(`Jobs::ensure_idle`). 같은 리소스 동시 받기는 네트워크 전에 거부("이미 받는 중입니다.").
- 오류 문구: "인터넷 연결이 필요합니다.", "받은 파일이 손상되었습니다. 다시 받아 주세요.", "리소스 이름이 올바르지 않습니다."
- DLL은 디스크에 쓰지 않는다(읽기만, 캐시는 메모리).
- 작업대 배경은 이 도구에서 테마 바탕(`stage-plain`). UI는 docs/UI-SYSTEM.md 토큰만. 한 파일 = 한 컴포넌트. 한국어 문구.
- ABI 버전은 올리지 않는다(새 도구는 새 앱 카탈로그에만 있음). GitHub 릴리스 태그 push는 이 계획에 없다.
- 의존성 추가 시 이유·라이선스를 커밋 메시지에.
- 매 Task 끝: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`(Rust 변경 시), `cd app && npm test && npx tsc --noEmit`(프론트 변경 시).

## Review Focus

- 세트를 바꿨는데 예전 `selected`가 남아 빈 패널·엉뚱한 아이콘 → Task 6 `set_option_resets` 테스트, Task 5 `unknown_selected_gives_no_panel_blocks`.
- 받기·삭제 뒤 같은 옵션이라 대화형 결과가 캐시에 머묾 → Task 7 리소스 mutation이 `interactiveKeys.all` 무효화(테스트는 훅 단위로 무효화 호출 확인).
- 받은 세트 폴더가 지워졌거나 `installed.json`이 깨짐 → Task 3 `missing_folder_is_skipped`, `broken_installed_json_is_empty`; Task 5 `missing_resource_dir_shows_download_text`.
- 큰 세트(Fluent 20k)에서 슬라이더를 움직일 때마다 전체 격자 재전송 → Task 5는 `limit`개만 보내고, DLL 캐시로 재파싱 없음(`second_run_uses_cache` 테스트).
- 악성 이름·body → Task 4 `drops_invalid_names`, `sanitize_*` 테스트 이식.

---

### Task 1: 1판 앱 내장 부분 걷어내기 + 카탈로그를 추가 설치 도구로

**Files:**
- Delete: `app/src/entities/icon-set/`, `app/src/features/icon-set-manage/`, `app/src/widgets/icon-library/`
- Modify: `app/src/pages/home/ui/home-page.tsx` (아이콘 라이브러리 분기 제거)
- Modify: `crates/builtin/src/lib.rs` (`CatalogEntry.screen` 필드·`screen_tools_are_not_builtin_or_addon` 테스트 제거)
- Modify: `catalog/tools.json` (`web.icon_library`에서 `"screen": true` 제거, `builtin: false` 유지)
- Modify: `app/src-tauri/src/commands.rs` (`list_tools`·`install_tool`의 `screen` 처리 제거, 아이콘 명령 6개 제거)
- Modify: `app/src-tauri/src/lib.rs` (아이콘 명령·`IconSets` manage 제거)
- Keep (다음 Task가 옮김): `app/src-tauri/src/addons/icons/*` — 이 Task에선 `pub mod icons;`를 남기되 쓰는 곳이 없어 생기는 dead_code는 모듈 상단 `#![allow(dead_code)]` + "Task 3에서 resources로 옮김" 주석.

**Interfaces:**
- Produces: `web.icon_library`가 `list_tools`에서 일반 추가 도구처럼 `notInstalled`.

- [ ] **Step 1: 실패하는 테스트** — `crates/builtin` `catalog_tests`에:

```rust
#[test]
fn icon_library_is_addon() {
    let e = catalog_all().into_iter().find(|e| e.id == "web.icon_library").unwrap();
    assert!(!e.builtin && !e.planned && !e.ai);
}
```
그리고 `screen` 필드 참조를 지워 컴파일 실패 확인.
- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools-builtin` (screen 테스트 삭제 전후).
- [ ] **Step 3: 구현** — 위 파일 변경. `tool-icons.ts`의 `"web.icon_library": IconIcons`는 유지.
- [ ] **Step 4: 통과 확인** — Rust 전체 + `npm test` + `tsc`.
- [ ] **Step 5: 커밋** — `refactor: 1판 아이콘 라이브러리 내장 화면 제거, 추가 설치 도구로 전환`

---

### Task 2: core `Block`에 `resourceList`·`iconGrid` 추가 (Rust + TS 타입)

**Files:**
- Modify: `crates/core/src/tool.rs` (`Block` 변형 2개 + 보조 타입, 직렬화 테스트)
- Modify: `app/src/entities/tool/model/types.ts` (`Block` 유니온)

**Interfaces:**
- Produces (serde camelCase, `tag = "type"`):

```rust
ResourceList {
    items: Vec<ResourceItem>,          // ResourceItem { id, title, meta: String }
    /// 받은 항목을 누르면 이 옵션을 항목 id로
    option: String,
    /// 그 옵션과 함께 비울 옵션들
    #[serde(default)] resets: Vec<String>,
    /// 지금 고른 id
    #[serde(default, skip_serializing_if = "Option::is_none")] selected: Option<String>,
},
IconGrid {
    cells: Vec<GridCell>,              // GridCell { key, svg, title: String }
    option: String,                    // 셀 클릭 → 이 옵션 = key, 같은 key면 ""(해제)
    #[serde(default, skip_serializing_if = "Option::is_none")] selected: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] chips: Option<GridChips>,   // { option, choices: Vec<Choice{value,label}>, value, resets: Vec<String> }
    #[serde(default, skip_serializing_if = "Option::is_none")] search: Option<GridSearch>, // { option, value, placeholder, resets: Vec<String> }
    #[serde(default, skip_serializing_if = "Option::is_none")] more: Option<GridMore>,     // { option, value: u32, label }
    /// 거른 전체 개수 (cells.len()보다 클 수 있음)
    total: u32,
},
```
TS 타입은 같은 필드(Option → `?:`).

- [ ] **Step 1: 실패하는 테스트** — `tool.rs` 테스트:

```rust
#[test] fn resource_list_round_trip() { /* to_value → v["type"]=="resourceList", v["items"][0]["id"]=="tabler", v["resets"][0]=="selected"; from_value 왕복 */ }
#[test] fn icon_grid_optional_fields_skip() { /* chips/search/more/selected None → JSON에 키 없음, total 있음 */ }
```
- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools-core` 컴파일 실패.
- [ ] **Step 3: 구현.**
- [ ] **Step 4: 통과 확인** — Rust 전체(다른 도구의 `Block` match가 깨지지 않는지), `tsc`.
- [ ] **Step 5: 커밋** — `feat: 결과 블록 resourceList·iconGrid 추가`

---

### Task 3: 앱 공용 "도구 리소스" 모듈 + 명령 + `__resources` 주입

**Files:**
- Move/rename: `app/src-tauri/src/addons/icons/{npm,unpack,store,mod}.rs` → `app/src-tauri/src/addons/resources/{npm,unpack,store,mod}.rs`; `sanitize.rs`·`set.rs`와 관련 테스트는 Task 4로 넘기기 위해 이 Task에서 `app` 쪽에서 삭제(내용은 Task 4가 git 기록에서 가져옴: `git show HEAD~:app/src-tauri/src/addons/icons/sanitize.rs`).
- Create: `app/src-tauri/src/addons/resources/spec.rs` (스키마 `x-resources` 읽기)
- Modify: `app/src-tauri/src/addons/mod.rs` (`uninstall`이 `addon-data/<id>` 삭제), `commands.rs`, `lib.rs`, `net.rs` 주석
- Modify: `CLAUDE.md` (도구 전용 UI 예외 줄 삭제, 네트워크 줄: "추가 도구 설치 모듈(도구 리소스 받기 포함, `registry.npmjs.org`, 사용자가 누를 때만)")
- Test: `app/src-tauri/src/addons/resources/tests.rs`, fixtures `tests/fixtures/icons/package/*` 재사용

**Interfaces:**
- Produces:
  - `spec::ResourceSpec { package: String /* "{id}" 포함 */, files: Vec<String> }`, `spec::parse(schema: &Value) -> Option<ResourceSpec>` — `x-resources.source == "npm"`, `package`에 `{id}` 정확히 1개, `files`는 비지 않고 각 항목에 `/`·`..` 없음.
  - `npm::valid_id(&str) -> bool` (1판 `valid_prefix` 이름 변경), `npm::package_for(spec, id) -> String`.
  - `ToolResources::from_env()` (root = `app_data_dir().join("addon-data")`), `::open(root)`.
  - `list(&self, tool_id) -> Vec<ResourceState>`; `ResourceState { id, version, latest: Option<String> }`.
  - `async install(&self, tool_id, spec: &ResourceSpec, id) -> Result<ResourceState, String>` — 1판 순서 그대로(Busy → 메타 → safe_name → tarball → `apply`), 저장 위치 `<root>/<tool_id>/<id>/<version>/<files>`, 기록 `<root>/<tool_id>/installed.json` `{"items": {"<id>": {"version": "..."}}}`.
  - `async check(&self, tool_id, spec) -> Result<Vec<ResourceState>, String>`, `remove(&self, tool_id, id) -> Result<(), String>`, `remove_tool(&self, tool_id)`.
  - `injected(&self, tool_id) -> Value` → `{"dir": "<root>/<tool_id>", "installed": {"<id>": "<version>"}}`.
  - 명령: `tool_resources(toolId) -> Vec<ResourceState>`, `install_tool_resource(toolId, id)` (`ensure_idle` 먼저, 도구가 로드돼 있고 `x-resources`가 있어야 함, 아니면 "도구를 찾을 수 없습니다."), `check_tool_resources(toolId)`, `remove_tool_resource(toolId, id)`.
  - `run_interactive`: 도구 스키마에 `x-resources`가 있으면 `opts["__resources"] = injected(tool_id)`를 넣고 실행. `uninstall_tool`은 `remove_tool`도 호출.
  - `tool_id`는 카탈로그에 있는 id만(경로에 쓰므로 `safe_name` 확인).

- [ ] **Step 1: 실패하는 테스트** (1판 icons 테스트를 리소스용으로 옮기고 아래 추가):

```rust
#[test] fn spec_parses_and_rejects() // 정상 / {id} 없음 / {id} 두 번 / files에 "../x" → None
#[test] fn install_writes_under_tool_dir()   // install_from → <root>/web.icon_library/test/1.0.0/icons.json 존재, list()==[test 1.0.0]
#[test] fn injected_lists_installed()        // {"dir": ..., "installed": {"test":"1.0.0"}}
#[test] fn remove_tool_deletes_folder()
#[test] fn invalid_id_is_rejected()          // "리소스 이름이 올바르지 않습니다."
// 이식: bad_integrity_is_rejected, update_replaces_and_removes_old, failed_update_keeps_old_version,
//       install_twice_is_rejected, broken_installed_json_is_empty, missing_folder_is_skipped
```
- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools resources`.
- [ ] **Step 3: 구현.** 동기 디스크 읽기를 하는 명령은 `spawn_blocking`.
- [ ] **Step 4: 통과 확인** — Rust 전체.
- [ ] **Step 5: 커밋** — `feat: 공용 도구 리소스 받기(npm) — 1판 아이콘 세트 받기를 도구 공용으로`

---

### Task 4: DLL 크레이트 뼈대 — 세트 목록 스냅샷 · 정리 · 세트 읽기

**Files:**
- Create: `crates/tools/web-icon-library/Cargo.toml` (`waple-tools-tool-web-icon-library`, feature `addon`, deps: core, addon-abi, serde, serde_json, schemars, base64, resvg `0.49` default-features=false(텍스트 없음), dev: tempfile, addon-abi testkit)
- Create: `src/lib.rs`(모듈 선언만, Tool은 Task 5), `src/collections.rs`, `src/sanitize.rs`, `src/set.rs`, `src/tests.rs`
- Create: `crates/tools/web-icon-library/data/collections.json` (스냅샷), `scripts/update-icon-collections.mjs`
- Modify: workspace `Cargo.toml` members는 `crates/tools/*`로 이미 포함 — 확인만

**Interfaces:**
- Produces:
  - `collections::Collection { id, name, total: u32, category: Option<String>, license: String, palette: bool }`, `collections::all() -> &'static [Collection]` (`include_str!("../data/collections.json")`를 `OnceLock`으로 한 번 파싱, 이름순).
  - `scripts/update-icon-collections.mjs`: `node scripts/update-icon-collections.mjs` — npm `@iconify/collections` 최신 tarball의 `collections.json`을 읽어 필터(Global Constraints) 후 위 구조 배열을 이름순으로 `data/collections.json`에 쓴다. (개발 시에만 실행, 앱·DLL은 실행하지 않음)
  - `sanitize::sanitize_body(&str) -> Option<String>` — 1판 그대로(`usvg::roxmltree` 사용으로 바꿈).
  - `set::IconSet { id, name, version, palette, width: f64, height: f64, suffixes: BTreeMap<String,String>, icons: Vec<Icon{ name, body, width: Option<f64>, height: Option<f64> }> }`.
  - `set::load(dir: &Path, id: &str, version: &str) -> Result<Arc<IconSet>, String>` — `<dir>/<id>/<version>/`의 세 파일 → 1판 `build_set` 규칙(이름 검증, 정리 후 빈 body 제외, 이름순, 크기 기본 16) → `(id, version)` 키로 `OnceLock<Mutex<HashMap<..>>>` 캐시.

- [ ] **Step 1: 실패하는 테스트** (1판 sanitize·build_set 테스트 이식 + 추가):

```rust
#[test] fn collections_snapshot_is_free_and_sorted() // 모두 4개 라이선스 중 하나, id 규칙 통과, 이름순, "tabler"·"ri" 포함
#[test] fn load_reads_fixture_set()                  // tempdir에 tests/fixtures/icons/package 세 파일 복사 → load 결과 이름 목록·suffixes
#[test] fn drops_invalid_names()
#[test] fn drops_empty_bodies()
#[test] fn second_run_uses_cache()                   // load 후 파일 삭제 → 같은 (id,version) load 성공(Arc::ptr_eq)
// sanitize_* 이식 (script·on*·외부 참조·CSS 이스케이프·image-set·네임스페이스 속성)
```
- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools-tool-web-icon-library`.
- [ ] **Step 3: 구현** — 스크립트 실행해 스냅샷 생성 후 커밋.
- [ ] **Step 4: 통과 확인.**
- [ ] **Step 5: 커밋** — `feat: 아이콘 라이브러리 도구 크레이트 — 세트 목록·정리·세트 읽기` (resvg 추가 이유·라이선스, Cargo.lock에 이미 있음)

---

### Task 5: DLL 도구 — 옵션 · 블록 · SVG/JSX/Data URI/클래스 · PNG

**Files:**
- Modify: `crates/tools/web-icon-library/src/lib.rs` (Tool impl, 옵션, `export_tool!`)
- Create: `src/styles.rs`, `src/render.rs`, `src/png.rs`, `src/blocks.rs`; 테스트 `src/tests.rs`에 추가

**Interfaces:**
- Consumes: Task 2 블록, Task 4 `collections`·`set`.
- Produces:
  - `ToolMeta { id: "web.icon_library", name: "아이콘 라이브러리", category: "web", extensions: [] }`, `ToolKind::Interactive`.
  - 옵션(schemars, camelCase, 스키마 루트 `x-input: "none"`, `x-resources` {source npm, package `@iconify-json/{id}`, files 세 개}, `x-stage: "list-detail"`, `x-stage-plain: true`, `x-panel-when: "selected"`, `x-panel-blocks: ["image","code"]`):
    - `set`, `style`, `query`, `selected`: String 기본 "" + `x-hidden`
    - `limit`: u32 기본 400 + `x-hidden`
    - `size: Option<u32>` 범위 8~1024 (비면 아이콘 높이) — "크기(px)"
    - `stroke: Option<f64>` 범위 0.5~3, `x-step` 0.25 (비면 원래 두께) — "두께"
    - `useColor: bool` 기본 false — "색 지정" (끄면 currentColor)
    - `color: String` 기본 "#000000", `format: color`, `x-enabled-when: { useColor: [true] }`
    - `pngScale`: enum `x1`·`x2`·`x4` 기본 `x2` — "PNG 배율"
    - `__resources`: 스키마에 없음(앱이 넣음). 없으면 받은 세트 없음으로 본다.
  - `styles::styles_of(set) -> BTreeMap<String,String>` (`tabler`면 `{"": "선", "filled": "채움"}`, 아니면 `suffixes`), `styles::style_of(name, &styles) -> &str`, `styles::filter(icons, query, style: Option<&str>, &styles) -> Vec<&Icon>` (1판 규칙: `""` 키 없으면 접미사 없는 아이콘은 모든 칩, 키 1개 이하면 거르지 않음, 검색은 공백 단위 단어 모두 포함·대소문자 무시).
  - `render::svg(icon, set, size, stroke, color) -> String`, `render::jsx(&str) -> String`, `render::data_uri(&str) -> String`, `render::class_snippet(set_id, name) -> Option<String>` (`tabler`·`ri`만) — 1판 TS 함수와 같은 출력.
  - `png::render(svg: &str, scale: u32) -> Result<Vec<u8>, ToolError>` (resvg, 텍스트 없음).
  - `blocks::build(opts, resources) -> Vec<Block>`: Spec §6 순서. `resourceList { option: "set", resets: ["selected","query","style","limit"], selected }`, 안 받은 세트를 고르면 `Text { "목록에서 [받기]를 누르면 아이콘을 볼 수 있습니다." }`, `iconGrid { option: "selected", chips {option "style", resets ["limit"]}, search {option "query", placeholder "아이콘 검색", resets ["limit"]}, more {option "limit", value limit+400, label "더 보기"} (남은 게 있을 때), total }`, `selected`가 거른 결과에 없으면 패널 블록 없음.
  - 셀 `svg`는 24px·원래 두께·currentColor.

- [ ] **Step 1: 실패하는 테스트:**

```rust
#[test] fn schema_flags()                       // s["x-panel-when"]=="selected", s["x-resources"]["package"]=="@iconify-json/{id}", 숨김 옵션들 x-hidden
#[test] fn no_resources_lists_collections_only() // __resources 없음 → blocks.len()==1, type resourceList, items 첫 항목 = collections::all()[0]
#[test] fn missing_resource_dir_shows_download_text() // set="test", installed 없음 → [resourceList, text]
#[test] fn grid_respects_limit_and_more()       // fixture 세트, limit=2 → cells 2, more.value==402 (limit+400), total==4
#[test] fn unsuffixed_icons_show_in_every_chip() // ri 모양 styles {line,fill} + "bold" → style "line"·"fill" 둘 다 포함
#[test] fn selected_produces_panel_blocks()     // selected="home" → image(name "home.png") + code svg(name "home.svg") + jsx + data uri (+ class 없음: set "test")
#[test] fn unknown_selected_gives_no_panel_blocks()
#[test] fn svg_output_matches_v1()              // size 48, stroke 1.5, currentColor → 1판 TS 기대 문자열과 같음
#[test] fn png_size_scales()                    // 24px, x2 → PNG 헤더 48×48
#[test] fn class_snippets()                     // tabler/home → <i class="ti ti-home"></i>, ri/home-line → <i class="ri-home-line"></i>, 그 외 None
```
- [ ] **Step 2: 실패 확인.**
- [ ] **Step 3: 구현.** `catalog/tools.json`의 이름이 meta와 같은지 확인(카탈로그 테스트가 DLL 도구는 검사하지 않으면 테스트 하나 추가: 이 크레이트 테스트에서 `include_str!("../../../../catalog/tools.json")`로 이름 비교).
- [ ] **Step 4: 통과 확인** — `cargo rustc -p waple-tools-tool-web-icon-library --features addon --crate-type cdylib` 도 한 번 성공(백그라운드).
- [ ] **Step 5: 커밋** — `feat: 아이콘 라이브러리 도구 — 목록·격자·패널 블록, SVG·PNG·코드 생성`

---

### Task 6: 프론트 공용 — 스키마 플래그 · 리소스 엔티티

**Files:**
- Modify: `app/src/features/tool-options/lib/schema-flags.ts` (+test): `stageMode(schema): "list-detail" | null` (`x-stage`), `isStagePlain(schema)` (`x-stage-plain`), `panelWhen(schema): string | null` (`x-panel-when`), `panelBlockTypes(schema): Block["type"][]` (`x-panel-blocks`, 없고 `x-result-panel: true`면 `["code"]`, 둘 다 없으면 `[]`)
- Modify: `app/src/features/tool-options/lib/schema-to-fields.ts` (+test): `x-hidden: true` 필드는 목록에서 빼되 기본값은 `initialOptions`에 들어가게; 숫자 슬라이더는 `x-step`이 있으면 그 값
- Create: `app/src/entities/tool-resource/` — `model/types.ts` (`ResourceState`), `api/tool-resource-api.ts` (명령 4개 래퍼), `api/tool-resource-keys.ts` (`all`, `list(toolId)`), `api/use-tool-resources.ts`, `api/use-install-resource.ts`, `api/use-remove-resource.ts`, `api/use-check-resources.ts`, `index.ts`
- Create: `app/src/features/interactive-result/lib/apply-option.ts` (+test)

**Interfaces:**
- Produces:
  - `applyOption(opts: Record<string, unknown>, key: string, value: unknown, resets: string[] = [], defaults: Record<string, unknown>): Record<string, unknown>` — key 설정, resets는 defaults 값으로(없으면 삭제).
  - mutation 훅 성공 시 `toolResourceKeys.list(toolId)`와 `interactiveKeys.all` 무효화, toast(`"받았습니다."`, `"삭제했습니다."`, 업데이트 확인은 새 버전 없으면 `"모두 최신입니다."`), 실패 `toast.error(toUserMessage(err))`.
  - mutation 훅은 엔티티에 둔다 (interactive-result 기능의 블록 뷰어가 써야 하므로 features 간 import를 피함).

- [ ] **Step 1: 실패하는 테스트:**

```ts
expect(panelBlockTypes({ "x-panel-blocks": ["image", "code"] })).toEqual(["image", "code"])
expect(panelBlockTypes({ "x-result-panel": true })).toEqual(["code"])
expect(panelWhen({ "x-panel-when": "selected" })).toBe("selected")
// schema-to-fields: x-hidden 필드는 fields에 없음, initialOptions에는 기본값 있음; x-step 0.25 → slider step 0.25
// apply-option (set_option_resets):
expect(applyOption({ set: "a", selected: "home", limit: 800 }, "set", "b", ["selected", "limit"], { selected: "", limit: 400 }))
  .toEqual({ set: "b", selected: "", limit: 400 })
```
- [ ] **Step 2: 실패 확인** — `npx vitest run src/features/tool-options src/features/interactive-result src/entities/tool-resource`.
- [ ] **Step 3: 구현.**
- [ ] **Step 4: 통과 확인** — `npm test`, `tsc`.
- [ ] **Step 5: 커밋** — `feat: 스키마 플래그(숨김·단위·작업대 배치·패널 조건)와 도구 리소스 엔티티`

---

### Task 7: 블록 뷰어 2종 + 대화형 작업대 연결

**Files:**
- Create: `app/src/features/interactive-result/ui/resource-list-block.tsx`, `resource-row.tsx`, `icon-grid-block.tsx`, `icon-grid-cell.tsx`; `lib/scope-ids.ts`(+test, 1판에서 이식)
- Modify: `app/src/features/interactive-result/ui/block-list.tsx` (`toolId`, `onSetOption?: (key, value, resets) => void` props, 새 블록 렌더)
- Modify: `app/src/widgets/tool-workspace/ui/interactive-tool-workspace.tsx` (플래그 적용)

**Interfaces:**
- Consumes: Task 2 타입, Task 6 플래그·`applyOption`·리소스 훅.
- 동작:
  - `ResourceListBlock`: 위 검색 Input(앱 안 필터, 제목·id), 행 = 제목 + meta(작은 회색) + 오른쪽 버튼(받기 전 [받기] 아이콘 버튼 / 받는 중 / 받음이면 행 클릭으로 선택, 새 버전 있으면 [업데이트], 메뉴에 [삭제]). 고른 행 강조. 아래 [업데이트 확인](받은 리소스가 없으면 비활성).
  - `IconGridBlock`: 위 줄 = 칩(Segmented, 선택지 2개 이상일 때) + 검색 Input + "N개", 아래 = 셀 격자, 끝에 [더 보기]. 셀 = 정사각형 카드(옅은 테두리, 둥근 모서리, 테마 바탕, 아이콘 가운데 24px) + 카드 아래 이름(작은 `text-muted-foreground`, 한 줄 말줄임), 열 수는 폭에 맞춰 자동(`grid-cols-[repeat(auto-fill,minmax(…))]`는 토큰 규칙상 허용되는 Tailwind 유틸리티로; 임의 색 금지), 고른 셀은 테두리 강조. 셀 클릭 → `onSetOption(option, key === selected ? "" : key)`. svg는 `scopeIds(svg, "c-" + key)` 후 `dangerouslySetInnerHTML`(주석: DLL에서 정리·이름 검증됨). 셀 컴포넌트는 `memo`.
  - 작업대: `isStagePlain`이면 `stage-plain`; `stageMode === "list-detail"`이면 `resourceList` 블록을 왼쪽 열(고정 폭, 세로 스크롤), 나머지 작업대 블록을 오른쪽에. `panelWhen` 옵션 값이 비면 옵션 패널(aside) 자체를 그리지 않음. 패널로 가는 블록은 `panelBlockTypes`(기존 `x-result-panel` 동작 유지).
  - `onSetOption`은 `setOptions(o => applyOption(o, key, value, resets, defaults))` — defaults는 `initialOptions(fields)`의 값 + 숨김 필드 기본값.
- [ ] **Step 1: 실패하는 테스트** — `scope-ids.test.ts` 이식; 블록 렌더 로직 중 순수 부분(`splitStageBlocks(blocks, mode, panelTypes) -> { list, stage, panel }`)을 `widgets/tool-workspace/lib/split-stage-blocks.ts`로 두고 테스트:

```ts
expect(splitStageBlocks(blocks, "list-detail", ["image", "code"]))
  .toEqual({ list: [resourceListBlock], stage: [gridBlock, textBlock], panel: [imageBlock, codeBlock] })
expect(splitStageBlocks([code], null, ["code"])).toEqual({ list: [], stage: [], panel: [code] })
```
- [ ] **Step 2: 실패 확인.**
- [ ] **Step 3: 구현.**
- [ ] **Step 4: 통과 확인** — `npm test`, `tsc`, `npm run build`.
- [ ] **Step 5: 커밋** — `feat: resourceList·iconGrid 블록 뷰어와 작업대 배치·패널 조건`

---

### Task 8: 전체 확인 (컨트롤러)

- [ ] **Step 1:** fmt · clippy · `cargo test --workspace` · `npm test` · `tsc` · `npm run build`.
- [ ] **Step 2:** 로컬 index로 설치 확인: DLL을 `cargo rustc --release -p waple-tools-tool-web-icon-library --features addon --crate-type cdylib`로 빌드(백그라운드), 개발용 키로 서명한 로컬 `index.json`을 만들어 `WAPLE_ADDON_INDEX`·`WAPLE_ADDON_PUBKEY`로 `npm run tauri dev`(백그라운드, `CARGO_TERM_COLOR=never`).
- [ ] **Step 3:** 사용자 수동 확인 목록 전달: 설치 전 [설치] 화면이 다른 추가 도구와 같음 / 설치 후 진입만으로 네트워크 없음 / 목록 [받기] → 격자(카드 모양, 테마 바탕) → 아이콘 선택 시에만 패널 → 복사 6종 / 다시 클릭하면 패널 닫힘 / 네트워크 끊고 재시작 후 검색·복사 / 도구 제거 시 받은 세트 폴더 삭제.
