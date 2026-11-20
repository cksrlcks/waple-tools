# 아이콘 라이브러리 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 웹폰트 탭에 Iconify 세트를 받아 두고 검색·조절·복사하는 "아이콘 라이브러리" 전용 화면을 만든다.

**Architecture:** src-tauri `addons/icons/`가 npm(`registry.npmjs.org`)에서 `@iconify-json/<prefix>` tarball을 받아 sha512 확인 → JSON 3개만 꺼내 → SVG body 정리 → `app_data_dir()/icons/<prefix>/<version>/set.json`에 저장한다. 프론트는 `set.json`을 읽어 검색·격자·오른쪽 패널·복사를 모두 처리한다. 카탈로그에 `screen` 플래그를 더해 DLL·내장 Tool 없이 사이드바에 "쓸 수 있음"으로 보인다.

**Tech Stack:** Rust(Tauri 2, reqwest, `tar` 0.4, `flate2` 1.1, `roxmltree` 0.21, `sha2` 0.10 — 모두 Cargo.lock에 이미 있음), React 19 + TanStack Query + Tailwind(UI-SYSTEM 토큰) + Vitest.

**Spec:** [docs/superpowers/specs/2026-11-20-icon-library-design.md](../specs/2026-11-20-icon-library-design.md)

## Global Constraints

- 네트워크는 사용자가 [받기]·[세트 더 보기]·[업데이트 확인]·[업데이트]를 누를 때만. 앱 시작·화면 진입 시 요청 금지.
- 허용 호스트: 기존 `github.com`, `*.githubusercontent.com` + `registry.npmjs.org`. https만.
- prefix 정규식 `^[a-z0-9]+(-[a-z0-9]+)*$`. 버전은 기존 `store::safe_name` 통과해야 폴더로 쓴다.
- 무료 라이선스 필터: spdx `MIT`, `Apache-2.0`, `ISC`, `CC0-1.0`, `hidden`이 아닌 것.
- 배치 작업 중에는 받기·업데이트 금지 (`Jobs::ensure_idle`).
- 화면 문구·오류는 한국어. 오류: "인터넷 연결이 필요합니다.", "받은 파일이 손상되었습니다. 다시 받아 주세요.", "세트 이름이 올바르지 않습니다."
- UI는 docs/UI-SYSTEM.md 토큰만 (`#hex`, `bg-[…]`, `text-gray-500` 금지). 모달 닫기는 하단 버튼, 상단 X 금지. 한 파일 = 한 컴포넌트.
- 의존성 추가 시 이유·라이선스를 커밋 메시지에.
- 끝나면 `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cd app && npm test && npx tsc --noEmit` 통과.

## Review Focus

- 세트 데이터가 큰 경우(Fluent 20,015개, Material Symbols 15,806개): 격자가 멈추지 않아야 한다 → Task 7의 400개 단위 페이지 + 검색, 수동 확인 항목.
- 같은 세트를 두 번 연달아 [받기]: 두 번째는 "이미 받는 중입니다." → Task 4 `install_twice_is_rejected`.
- 업데이트 도중 실패(해시 불일치): 기존 버전이 그대로 남아 계속 쓸 수 있어야 한다 → Task 4 `failed_update_keeps_old_version`.
- 접두사가 겹치는 스타일 이름(`academic-cap-20-solid`가 `solid`가 아니라 `20-solid`) → Task 5 `styleOf` 테스트.
- `installed.json`이 깨졌거나 기록된 폴더가 지워진 경우: 빈 목록/그 세트만 빠지고 앱은 정상 → Task 4 `broken_installed_json_is_empty`, `missing_folder_is_skipped`.

---

### Task 1: 카탈로그 `screen` 플래그와 CLAUDE.md 규칙

**Files:**
- Modify: `crates/builtin/src/lib.rs` (`CatalogEntry`, `catalog_tests`)
- Modify: `catalog/tools.json` (웹폰트 항목 끝, `web.font_subset` 다음)
- Modify: `app/src-tauri/src/commands.rs` (`list_tools`, `install_tool`)
- Modify: `app/src/entities/tool/config/tool-icons.ts`
- Modify: `CLAUDE.md`

**Interfaces:**
- Produces: `CatalogEntry.screen: bool` (`#[serde(default)]`, 문서 주석 "앱 전용 화면 (DLL·내장 Tool 없음)"). `list_tools`는 `screen` 항목을 `ToolStatus::Builtin`, `kind: None`으로 낸다. 프론트는 `tool.id === "web.icon_library"`로 화면을 고른다 (Task 7).

- [ ] **Step 1: 실패하는 테스트** — `catalog_tests`에 추가:

```rust
#[test]
fn screen_tools_are_not_builtin_or_addon() {
    let screens: Vec<_> = catalog_all().into_iter().filter(|e| e.screen).collect();
    assert_eq!(screens.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["web.icon_library"]);
    assert!(screens.iter().all(|e| !e.builtin && !e.planned && !e.ai));
}
```

- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools-builtin screen_tools` → 컴파일 오류(`screen` 없음).
- [ ] **Step 3: 구현**
  - `CatalogEntry`에 `screen` 필드.
  - tools.json에 `{ "id": "web.icon_library", "name": "아이콘 라이브러리", "category": "web", "description": "무료 아이콘 세트를 받아 검색하고 SVG·PNG·클래스로 복사합니다.", "builtin": false, "screen": true }`.
  - `list_tools`: `info`는 `(!e.builtin && !e.screen)`일 때만, status 매치에 `(None, _) if e.screen => ToolStatus::Builtin`을 `planned` 앞에.
  - `install_tool`의 카탈로그 확인에 `&& !e.screen`.
  - `tool-icons.ts`: `"web.icon_library": IconIcons` (`@tabler/icons-react`).
  - CLAUDE.md: 네트워크 줄에 "아이콘 세트 받기(`app/src-tauri/src/addons/icons`, `registry.npmjs.org`, 사용자가 누를 때만)" 추가. 도구 전용 UI 줄에 "예외: `web.icon_library`(전용 화면, 카탈로그 `screen`)" 추가.
- [ ] **Step 4: 통과 확인** — `cargo test -p waple-tools-builtin` PASS, `cargo test -p waple-tools` PASS.
- [ ] **Step 5: 커밋** — `feat: 카탈로그 screen 플래그와 아이콘 라이브러리 항목`

---

### Task 2: npm 메타데이터 · sha512 · prefix · 허용 호스트

**Files:**
- Modify: `app/src-tauri/src/addons/net.rs` (`allowed`, `UNAVAILABLE` 문구, 테스트)
- Modify: `app/src-tauri/src/addons/mod.rs` (`pub mod icons;`, `net`을 `pub(crate)`로)
- Create: `app/src-tauri/src/addons/icons/mod.rs` (이 Task에선 `mod npm;`과 아래 재노출만)
- Create: `app/src-tauri/src/addons/icons/npm.rs`
- Modify: `app/src-tauri/Cargo.toml` (`tar = "0.4"`, `flate2 = "1"`, `roxmltree = "0.21"` — Task 3에서 쓰지만 여기서 한 번에. sha2는 이미 있음)

**Interfaces:**
- Produces (`icons::npm`):
  - `pub fn valid_prefix(s: &str) -> bool`
  - `pub fn latest_url(package: &str) -> String` → `https://registry.npmjs.org/{package}/latest`
  - `pub fn set_package(prefix: &str) -> String` → `@iconify-json/{prefix}`; `pub const COLLECTIONS_PACKAGE: &str = "@iconify/collections"`
  - `pub struct PackageMeta { pub version: String, pub tarball: String, pub integrity: String }` + `pub fn parse_meta(bytes: &[u8]) -> Option<PackageMeta>` (`version`, `dist.tarball`, `dist.integrity`)
  - `pub fn verify_integrity(bytes: &[u8], integrity: &str) -> bool` (`sha512-<base64>`만 인정)

- [ ] **Step 1: 실패하는 테스트** (`npm.rs` 안 `#[cfg(test)]`):

```rust
#[test] fn prefix_rules() {
    for ok in ["tabler", "ri", "material-symbols", "fluent-emoji-flat", "ic"] { assert!(valid_prefix(ok), "{ok}"); }
    for bad in ["", "-a", "a-", "a--b", "A", "../x", "a/b", "a.b", "a b"] { assert!(!valid_prefix(bad), "{bad}"); }
}
#[test] fn parses_latest_meta() {
    let m = parse_meta(br#"{"version":"1.2.46","dist":{"tarball":"https://registry.npmjs.org/@iconify-json/tabler/-/tabler-1.2.46.tgz","integrity":"sha512-AAAA"}}"#).unwrap();
    assert_eq!(m.version, "1.2.46");
    assert!(m.tarball.ends_with("tabler-1.2.46.tgz"));
}
#[test] fn integrity_sha512_only() {
    let b = b"hello";
    let good = format!("sha512-{}", base64::engine::general_purpose::STANDARD.encode(sha2::Sha512::digest(b)));
    assert!(verify_integrity(b, &good));
    assert!(!verify_integrity(b"other", &good));
    assert!(!verify_integrity(b, "sha1-abc"));
}
```

`net.rs` 테스트에 추가: `assert!(ok("https://registry.npmjs.org/@iconify-json/tabler/latest")); assert!(!ok("http://registry.npmjs.org/x")); assert!(!ok("https://registry.npmjs.org.evil.com/x"));`

- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools icons::npm` → 컴파일 오류.
- [ ] **Step 3: 구현** — 위 시그니처. `allowed`에 `h == "registry.npmjs.org"`. `UNAVAILABLE`을 "받을 수 없습니다. 잠시 뒤 다시 시도해 주세요."로 (아이콘에서도 쓰므로 "추가 도구 목록" 제거). 모듈 주석 첫 줄을 "추가 도구·아이콘 세트 다운로드"로.
- [ ] **Step 4: 통과 확인** — `cargo test -p waple-tools addons` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 세트용 npm 메타데이터·sha512 확인, registry.npmjs.org 허용` (본문: tar·flate2·roxmltree 추가 이유와 MIT/Apache-2.0, Cargo.lock에 이미 있어 새 크레이트 없음)

---

### Task 3: tarball 꺼내기 · SVG 정리 · 세트 만들기

**Files:**
- Create: `app/src-tauri/src/addons/icons/unpack.rs`
- Create: `app/src-tauri/src/addons/icons/sanitize.rs`
- Create: `app/src-tauri/src/addons/icons/set.rs`
- Create: `tests/fixtures/icons/package/icons.json`, `info.json`, `metadata.json`, `tests/fixtures/icons/collections.json`
- Create: `app/src-tauri/src/addons/icons/tests.rs` (+ `fn fixture_tgz(files: &[(&str, &[u8])]) -> Vec<u8>`: `tar::Builder` + `flate2::write::GzEncoder`로 테스트 중 tgz 생성)

**Fixture 내용:**
- `icons.json`: `prefix: "test"`, `width: 24, height: 24`, icons: `home`(stroke-width="2" path), `home-filled`(fill path), `small-20-solid`(`width: 20, height: 20`), `evil`(`<path d="M0 0" onclick="x()"/><script>alert(1)</script><image href="https://e.x/a.png"/><path fill="url(https://e.x/#a)" d="M1 1"/><path fill="url(#g)" d="M2 2"/>`), `broken`(`<path d="M0 0"`), aliases 1개.
- `info.json`: `name: "Test Icons", total: 5, license: {title: "MIT", spdx: "MIT"}`, `palette: false`.
- `metadata.json`: `{"suffixes": {"": "Outline", "filled": "Filled"}}`.
- `collections.json`: `mit-set`(MIT), `apache-set`(Apache-2.0), `isc-set`(ISC), `cc0-set`(CC0-1.0), `ccby-set`(CC-BY-4.0), `gpl-set`(GPL-3.0), `hidden-set`(MIT, `hidden: true`), 각 `name/total/category/license/palette`.

**Interfaces:**
- Produces:
  - `unpack::read_package_files(tgz: &[u8], names: &[&str]) -> Result<HashMap<String, Vec<u8>>, String>` — 항목 경로가 정확히 `package/<name>`인 것만 메모리로 읽는다 (디스크에 풀지 않음). 한 파일 64 MiB 초과는 오류.
  - `sanitize::sanitize_body(body: &str) -> Option<String>` — 허용 요소: `g path circle rect line polyline polygon ellipse defs clipPath mask linearGradient radialGradient stop`. 허용 속성: `d fill stroke stroke-width stroke-linecap stroke-linejoin stroke-miterlimit stroke-dasharray stroke-dashoffset stroke-opacity fill-opacity fill-rule clip-rule opacity transform cx cy r rx ry x y x1 y1 x2 y2 width height points id clip-path mask offset stop-color stop-opacity gradientUnits gradientTransform fx fy pathLength`. 값에 `url(`이 있으면 `url(#`로 시작하는 것만. 허용 밖 요소는 자식째 버린다. 파싱 실패면 None. `<svg xmlns="http://www.w3.org/2000/svg">…</svg>`로 감싸 `roxmltree`로 읽고 속성 값은 XML 이스케이프해 다시 쓴다.
  - `set::IconSet` (serde camelCase, `set.json`과 프론트 공통): `{ prefix, name, version, license: License { title, spdx: Option<String> }, palette: bool, width: u32, height: u32, suffixes: BTreeMap<String,String>, icons: Vec<Icon { name, body, width: Option<u32>, height: Option<u32> }> }`. 세트 width/height 없으면 16.
  - `set::build_set(prefix: &str, version: &str, files: &HashMap<String, Vec<u8>>) -> Result<IconSet, String>` — `icons.json`·`info.json` 필수, `metadata.json` 선택. aliases 무시. 정리 실패 아이콘은 뺀다. icons는 이름순.
  - `set::Collection { prefix, name, total: u32, category: Option<String>, license: String /*spdx*/, palette: bool }` + `set::free_collections(json: &[u8]) -> Result<Vec<Collection>, String>` — Global Constraints의 필터, prefix가 `valid_prefix`인 것만, 이름순.

- [ ] **Step 1: 실패하는 테스트** (`tests.rs`):

```rust
#[test] fn reads_only_requested_package_files() // fixture_tgz에 package/index.js·package/icons.json·other/icons.json → icons.json만, 내용은 package/의 것
#[test] fn sanitize_drops_script_handlers_and_external_refs() {
    let s = sanitize_body(EVIL_BODY).unwrap();
    assert!(!s.contains("script") && !s.contains("onclick") && !s.contains("<image") && !s.contains("https://"));
    assert!(s.contains(r#"fill="url(#g)""#) && s.contains(r#"d="M2 2""#));
}
#[test] fn sanitize_keeps_stroke_icon_as_is_semantically() // home body → stroke-width="2", stroke-linecap 등 유지
#[test] fn sanitize_rejects_broken_xml() { assert!(sanitize_body(r#"<path d="M0 0""#).is_none()); }
#[test] fn build_set_from_fixture() {
    let set = build_set("test", "1.0.0", &fixture_files()).unwrap();
    assert_eq!(set.name, "Test Icons"); assert_eq!(set.license.spdx.as_deref(), Some("MIT"));
    assert_eq!(set.icons.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), ["evil", "home", "home-filled", "small-20-solid"]);
    assert_eq!(set.suffixes.get("filled").map(String::as_str), Some("Filled"));
    assert_eq!(set.icons.iter().find(|i| i.name == "small-20-solid").unwrap().width, Some(20));
}
#[test] fn build_set_requires_icons_json() // icons.json 빠지면 Err
#[test] fn free_collections_filters_license_and_hidden() {
    let c = free_collections(&std::fs::read(fixture("collections.json")).unwrap()).unwrap();
    assert_eq!(c.iter().map(|c| c.prefix.as_str()).collect::<Vec<_>>(), ["apache-set", "cc0-set", "isc-set", "mit-set"]); // name이 prefix와 같은 순서가 되게 fixture 이름 지정
}
```

fixture 경로: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/icons")`.

- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools icons::tests` → 컴파일 오류.
- [ ] **Step 3: 구현** — 위 시그니처.
- [ ] **Step 4: 통과 확인** — `cargo test -p waple-tools icons` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 세트 tarball 읽기·SVG 정리·세트 변환`

---

### Task 4: 세트 저장소와 Tauri 명령

**Files:**
- Modify: `app/src-tauri/src/addons/icons/mod.rs` (`IconSets`)
- Create: `app/src-tauri/src/addons/icons/store.rs`
- Modify: `app/src-tauri/src/addons/icons/tests.rs`
- Modify: `app/src-tauri/src/commands.rs`, `app/src-tauri/src/lib.rs` (`.manage(IconSets::from_env())`, 명령 등록)

**Interfaces:**
- Consumes: Task 2 `npm::*`, Task 3 `unpack`/`set`, `net::fetch(url, None, |_, _| {})`, `store::safe_name`, `waple_tools_builtin::app_data_dir()`.
- Produces:
  - `icons::store`: `installed.json` = `{"sets": {"<prefix>": {"name": "...", "version": "..."}}}` (`BTreeMap`), `read_installed(root) -> Installed`(깨지면 빈 값), `write_installed(root, &Installed) -> io::Result<()>`(tmp→rename), `set_dir(root, prefix, version) -> PathBuf`, `write_set(dir, &IconSet) -> io::Result<()>`(`set.json`, tmp→rename).
  - `pub struct InstalledSet { prefix, name, version, latest: Option<String> }` (serde camelCase)
  - `IconSets::from_env()` (root = `app_data_dir().join("icons")`), `IconSets::open(root: PathBuf) -> Self`
  - `fn list(&self) -> Vec<InstalledSet>` — 기록 중 `set.json`이 있는 것만, 마지막 `check_updates` 결과의 `latest` 포함.
  - `async fn install(&self, prefix: &str) -> Result<InstalledSet, String>` — 받기와 업데이트 공용. 순서: `valid_prefix` → 진행 중 집합에 넣기(있으면 "이미 받는 중입니다.") → 메타 → `safe_name(version)` → tarball → `verify_integrity` → `read_package_files(["icons.json","info.json","metadata.json"])` → `build_set` → 새 버전 폴더에 `write_set` → `installed.json` 갱신 → 예전 버전 폴더 삭제. 중간 실패 시 새 폴더만 지우고 예전 기록·폴더 유지.
  - `async fn check_updates(&self) -> Result<Vec<InstalledSet>, String>` — 받은 세트마다 메타만 받아 `latest` 기록 후 `list()`.
  - `fn remove(&self, prefix: &str) -> Result<(), String>` — 기록 삭제 → `<root>/<prefix>` 폴더 삭제.
  - `fn read(&self, prefix: &str) -> Result<IconSet, String>` — 기록된 버전의 `set.json`. 없으면 "받은 세트가 없습니다."
  - `async fn collections(&self) -> Result<Vec<Collection>, String>` — `COLLECTIONS_PACKAGE` 메타 → tarball → integrity → `collections.json` → `free_collections`. 디스크에 쓰지 않음.
  - 네트워크 부분은 테스트를 위해 `async fn install_from(&self, prefix, meta: PackageMeta, tgz: Vec<u8>)`로 나눠 `install`이 받기 후 호출한다.
  - 명령 (`commands.rs`): `icon_sets() -> Vec<InstalledSet>`, `install_icon_set(prefix: String) -> Result<InstalledSet, String>`(먼저 `jobs.ensure_idle()?`), `check_icon_updates() -> Result<Vec<InstalledSet>, String>`, `remove_icon_set(prefix: String) -> Result<(), String>`, `read_icon_set(prefix: String) -> Result<IconSet, String>`, `icon_collections() -> Result<Vec<Collection>, String>`.

- [ ] **Step 1: 실패하는 테스트** (`tests.rs`, `tempfile::tempdir()` root):

```rust
#[tokio::test] async fn install_writes_set_and_record()      // install_from → list()==[test 1.0.0], read("test").icons.len()==4, set.json 존재
#[tokio::test] async fn bad_integrity_is_rejected()          // integrity 틀림 → Err("받은 파일이 손상되었습니다. 다시 받아 주세요."), list() 빔, root/test 없음
#[tokio::test] async fn update_replaces_and_removes_old()    // 1.0.0 → 1.1.0 → list() 버전 1.1.0, root/test/1.0.0 없음
#[tokio::test] async fn failed_update_keeps_old_version()    // 1.0.0 설치 후 1.1.0 integrity 틀림 → Err, read() 여전히 1.0.0, root/test/1.1.0 없음
#[tokio::test] async fn install_twice_is_rejected()          // 진행 중 집합에 미리 넣고 install_from → Err("이미 받는 중입니다.")
#[test] fn invalid_prefix_is_rejected()                      // install("../x") → Err("세트 이름이 올바르지 않습니다.") (네트워크 전)
#[test] fn remove_deletes_folder_and_record()
#[test] fn broken_installed_json_is_empty()                  // installed.json에 "{" → open 후 list() 빔
#[test] fn missing_folder_is_skipped()                       // 기록만 있고 폴더 없음 → list()에서 빠짐
```

`tokio`가 dev-dependency에 없으면 `tauri::async_runtime::block_on`으로 동기 테스트로 쓴다 (새 의존성 금지).

- [ ] **Step 2: 실패 확인** — `cargo test -p waple-tools icons` → 실패.
- [ ] **Step 3: 구현** — 위 인터페이스. 잠금은 기존 `addons::lock` 패턴(`Mutex` + poison 무시).
- [ ] **Step 4: 통과 확인** — `cargo test -p waple-tools` PASS, `cargo clippy --workspace -- -D warnings` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 세트 받기·업데이트·삭제 저장소와 Tauri 명령`

---

### Task 5: 프론트 엔티티 `icon-set` (타입·API·변환 함수)

**Files:**
- Create: `app/src/entities/icon-set/index.ts`
- Create: `app/src/entities/icon-set/model/types.ts`
- Create: `app/src/entities/icon-set/api/icon-set-api.ts`, `icon-set-keys.ts`, `use-installed-sets.ts`, `use-icon-set.ts`
- Create: `app/src/entities/icon-set/config/recommended.ts`
- Create: `app/src/entities/icon-set/lib/style-of.ts`, `render-svg.ts`, `copy-formats.ts` + 각 `.test.ts`

**Interfaces:**
- Consumes: Task 4 명령 이름·`IconSet`/`InstalledSet`/`Collection` JSON (camelCase).
- Produces:
  - 타입 `IconSet`, `IconItem`, `InstalledSet`, `Collection` (Rust와 같은 필드).
  - API: `listIconSets()`, `installIconSet(prefix)`, `checkIconUpdates()`, `removeIconSet(prefix)`, `readIconSet(prefix)`, `iconCollections()` (`invoke` 래퍼).
  - `iconSetKeys = { all: ["iconSets"], installed: () => [...all, "installed"], set: (prefix) => [...all, "set", prefix], collections: () => [...all, "collections"] }`.
  - `useInstalledSets()`(`listIconSets`, 로컬만), `useIconSet(prefix: string | null)`(`enabled: !!prefix`, `staleTime: Infinity`).
  - `RECOMMENDED: Recommended[]`, `type Recommended = { prefix: string; name: string; license: string; styles?: Record<string, string>; className?: (name: string) => string }`:
    - `tabler` "Tabler Icons" MIT, styles `{ "": "선", filled: "채움" }`, className `` n => `ti ti-${n}` ``
    - `heroicons` "Heroicons" MIT, styles `{ "": "선 24", solid: "채움 24", "20-solid": "채움 20", "16-solid": "채움 16" }`
    - `ri` "Remix Icon" Apache-2.0, styles `{ line: "선", fill: "채움" }`, className `` n => `ri-${n}` ``
  - `stylesOf(set: IconSet): Record<string, string>` — RECOMMENDED의 styles → `set.suffixes` → `{}`.
  - `styleOf(name: string, styles: Record<string, string>): string` — 빈 문자열이 아닌 키 중 `name.endsWith("-" + key)`인 가장 긴 키, 없으면 `""`.
  - `type RenderOpts = { size: number; stroke: number | null; color: string }` (`color`는 `"currentColor"` 또는 `#rrggbb`)
  - `iconSize(icon, set): { width: number; height: number }`, `strokeOf(body): number | null`(첫 `stroke-width` 값)
  - `toSvg(icon: IconItem, set: IconSet, opts: RenderOpts): string` — `<svg xmlns="http://www.w3.org/2000/svg" width="{size*w/h 비율}" height="{size}" viewBox="0 0 {w} {h}">{body}</svg>`; `stroke`가 있으면 모든 `stroke-width="…"` 교체; `color`가 `currentColor`가 아니면 `currentColor` 교체.
  - `toJsx(svg: string): string` — `\s([a-z]+(-[a-z]+)+)=` 속성을 camelCase로 (`data-`·`aria-` 제외).
  - `toDataUri(svg: string): string` — `` `url("data:image/svg+xml,${encodeURIComponent(svg)}")` ``
  - `toClassSnippet(prefix: string, name: string): string | null` — RECOMMENDED className이 있으면 `` `<i class="${cls}"></i>` ``, 없으면 null.

- [ ] **Step 1: 실패하는 테스트**

```ts
// style-of.test.ts
const hero = RECOMMENDED.find((r) => r.prefix === "heroicons")!.styles!
expect(styleOf("academic-cap-20-solid", hero)).toBe("20-solid")
expect(styleOf("academic-cap-solid", hero)).toBe("solid")
expect(styleOf("academic-cap", hero)).toBe("")
expect(styleOf("home-filled", { "": "선", filled: "채움" })).toBe("filled")
expect(stylesOf({ ...set, prefix: "other", suffixes: { line: "Line" } })).toEqual({ line: "Line" })
// render-svg.test.ts
const icon = { name: "home", body: '<path stroke="currentColor" stroke-width="2" d="M0 0"/>' }
const set = { width: 24, height: 24, /* … */ } as IconSet
expect(toSvg(icon, set, { size: 48, stroke: 1.5, color: "currentColor" })).toBe(
  '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24"><path stroke="currentColor" stroke-width="1.5" d="M0 0"/></svg>')
expect(toSvg(icon, set, { size: 24, stroke: null, color: "#ff0000" })).toContain('stroke="#ff0000" stroke-width="2"')
expect(iconSize({ ...icon, width: 20, height: 20 }, set)).toEqual({ width: 20, height: 20 })
expect(strokeOf('<path fill="x"/>')).toBeNull()
// copy-formats.test.ts
expect(toJsx('<path stroke-width="2" stroke-linecap="round" data-x="1"/>')).toBe('<path strokeWidth="2" strokeLinecap="round" data-x="1"/>')
expect(toDataUri("<svg/>")).toBe('url("data:image/svg+xml,%3Csvg%2F%3E")')
expect(toClassSnippet("tabler", "home")).toBe('<i class="ti ti-home"></i>')
expect(toClassSnippet("ri", "home-line")).toBe('<i class="ri-home-line"></i>')
expect(toClassSnippet("heroicons", "home")).toBeNull()
```

- [ ] **Step 2: 실패 확인** — `cd app && npx vitest run src/entities/icon-set` → FAIL.
- [ ] **Step 3: 구현** — 위 시그니처.
- [ ] **Step 4: 통과 확인** — 같은 명령 PASS, `npx tsc --noEmit` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 세트 엔티티 — 타입·API·스타일·SVG 변환`

---

### Task 6: 세트 관리 기능 (`features/icon-set-manage`)

**Files:**
- Create: `app/src/features/icon-set-manage/index.ts`
- Create: `api/use-install-set.ts`, `api/use-remove-set.ts`, `api/use-check-updates.ts`, `api/use-collections.ts`
- Create: `ui/set-download-panel.tsx`, `ui/collections-dialog.tsx`, `ui/collection-row.tsx`, `ui/set-actions-menu.tsx`
- Create: `lib/filter-collections.ts` + `.test.ts`

**Interfaces:**
- Consumes: Task 5 API·키·`RECOMMENDED`·타입.
- Produces:
  - `useInstallSet()` — mutation(`installIconSet`), 성공 시 `iconSetKeys.all` 무효화 + `toast.success("<이름> 세트를 받았습니다.")`, 실패 `toast.error(toUserMessage(err))`.
  - `useRemoveSet()` — 확인은 `useConfirm()`("세트를 삭제할까요?", "받은 아이콘 파일을 지웁니다. 다시 받을 수 있습니다."), 성공 무효화 + toast.
  - `useCheckUpdates()` — mutation, 결과로 `iconSetKeys.installed()` 캐시 갱신, 새 버전 없으면 `toast.success("모두 최신입니다.")`, 있으면 `toast.info("새 버전이 있는 세트 N개")`.
  - `useCollections(open: boolean)` — query, `enabled: open`(대화상자를 열 때만 네트워크), `staleTime: Infinity`, `retry: false`.
  - `filterCollections(list: Collection[], query: string): Collection[]` — 이름·prefix 대소문자 무시 포함 검색.
  - `<SetDownloadPanel prefix name license className? />` — 안 받은 세트 안내: 이름, 라이선스, "받으면 인터넷 없이 쓸 수 있습니다.", [받기](진행 중 비활성 + "받는 중…").
  - `<CollectionsDialog open onOpenChange installed: InstalledSet[] />` — 검색 Input, 행 목록(이름·분류·개수·라이선스·[받기]/"받음"), 오류 시 `ErrorBanner`, 하단 [닫기] 버튼(상단 X 없음).
  - `<SetActionsMenu set: InstalledSet />` — DropdownMenu: [업데이트](latest > version일 때), [삭제].
- [ ] **Step 1: 실패하는 테스트** — `filter-collections.test.ts`: `filterCollections([{prefix:"tabler",name:"Tabler Icons"}, {prefix:"ri",name:"Remix Icon"}], "REMIX")` → ri만, `""` → 전부.
- [ ] **Step 2: 실패 확인** — `npx vitest run src/features/icon-set-manage` FAIL.
- [ ] **Step 3: 구현** — 위 인터페이스. 컴포넌트는 `@/shared/ui`(Button, Dialog, Input, DropdownMenu, ErrorBanner)와 UI-SYSTEM 토큰만.
- [ ] **Step 4: 통과 확인** — vitest PASS, `npx tsc --noEmit` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 세트 받기·삭제·업데이트 확인·세트 목록 대화상자`

---

### Task 7: 아이콘 라이브러리 화면 (`widgets/icon-library`) + 연결

**Files:**
- Create: `app/src/widgets/icon-library/index.ts`
- Create: `ui/icon-library.tsx`, `ui/icon-toolbar.tsx`, `ui/icon-grid.tsx`, `ui/icon-cell.tsx`, `ui/icon-detail-panel.tsx`, `ui/copy-actions.tsx`
- Create: `model/use-icon-library-state.ts`, `model/use-paged-count.ts`
- Create: `lib/filter-icons.ts` + `.test.ts`, `lib/svg-to-png.ts`
- Create: `config/icon-library.ts` (`ICON_LIBRARY_ID = "web.icon_library"`, `PAGE_SIZE = 400`, `PNG_SCALES = [1, 2, 4]`)
- Modify: `app/src/pages/home/ui/home-page.tsx` (선택 도구가 `ICON_LIBRARY_ID`면 `<IconLibrary key=… />`, 아니면 `ToolWorkspace`)

**Interfaces:**
- Consumes: Task 5(`useInstalledSets`, `useIconSet`, `stylesOf`, `styleOf`, `toSvg`, `toJsx`, `toDataUri`, `toClassSnippet`, `iconSize`, `strokeOf`, `RECOMMENDED`), Task 6(`SetDownloadPanel`, `CollectionsDialog`, `SetActionsMenu`, `useCheckUpdates`), `@/features/interactive-result`의 저장은 쓰지 않고 `invoke("save_block", { path, content, base64: false })` + `@tauri-apps/plugin-dialog` `save`를 `lib/save-svg.ts`에서 직접 (features 간 import 금지).
- Produces:
  - `filterIcons(icons: IconItem[], opts: { query: string; style: string | null; styles: Record<string,string> }): IconItem[]` — `style`이 null이면 스타일 무시, 아니면 `styleOf(name, styles) === style`; query는 공백으로 나눈 단어가 모두 이름에 포함(대소문자 무시).
  - `svgToPngBlob(svg: string, px: { width: number; height: number }): Promise<Blob>` — data URI `<img>` → canvas → `toBlob("image/png")`.
  - `useIconLibraryState()` — `{ prefix, setPrefix, style, setStyle, query, setQuery, selected, setSelected, size, setSize, stroke, setStroke, color, setColor }`. 세트를 바꾸면 `selected=null`, `style`=첫 스타일 키(없으면 null). 아이콘을 고르면 `size`=그 아이콘 높이, `stroke`=`strokeOf(body)`.
  - 초기 세트: 받은 세트 첫 번째, 없으면 `RECOMMENDED[0].prefix`.
- 화면 동작 (spec 7장):
  - 툴바: 세트 Select(받은 세트 + 안 받은 추천 3종 "받기 전" 표시), 스타일 Segmented(키 1개 이하면 숨김), 검색 Input, "N개 · v버전", `SetActionsMenu`, [업데이트 확인], [세트 더 보기].
  - 안 받은 세트면 격자 자리에 `SetDownloadPanel`.
  - 격자: `PAGE_SIZE`개씩, 맨 아래 sentinel이 보이면(IntersectionObserver) 다음 페이지. 검색·세트·스타일이 바뀌면 처음부터. 셀은 `toSvg(icon, set, { size: 24, stroke: null, color: "currentColor" })`를 `dangerouslySetInnerHTML`로 (Rust에서 정리된 body만 들어온다는 주석). 선택 셀은 강조, `title`=이름.
  - 오른쪽 패널(선택 시에만, `w-80`): 큰 미리보기, 이름, 크기 Input(px, 8~1024), 두께 Slider(0.5~3, 0.25 단위, `strokeOf`가 null이면 숨김), 색(`palette`면 숨김, `currentColor` 토글 + `<input type="color">`), 복사 버튼: SVG / PNG 1x·2x·4x / 클래스(`toClassSnippet` null이면 숨김) / JSX / Data URI / SVG 저장(`<이름>.svg`). PNG는 `currentColor`면 `#000000`으로 렌더. 복사 성공 `toast.success("복사했습니다.")`, 실패 `toast.error("복사할 수 없습니다.")`.
- [ ] **Step 1: 실패하는 테스트** — `filter-icons.test.ts`:

```ts
const icons = ["home", "home-filled", "arrow-left", "arrow-left-filled"].map((name) => ({ name, body: "" }))
const styles = { "": "선", filled: "채움" }
expect(filterIcons(icons, { query: "", style: "", styles }).map((i) => i.name)).toEqual(["home", "arrow-left"])
expect(filterIcons(icons, { query: "LEFT arrow", style: "filled", styles }).map((i) => i.name)).toEqual(["arrow-left-filled"])
expect(filterIcons(icons, { query: "home", style: null, styles })).toHaveLength(2)
```

- [ ] **Step 2: 실패 확인** — `npx vitest run src/widgets/icon-library` FAIL.
- [ ] **Step 3: 구현** — 위 인터페이스.
- [ ] **Step 4: 통과 확인** — `npm test` PASS, `npx tsc --noEmit` PASS.
- [ ] **Step 5: 커밋** — `feat: 아이콘 라이브러리 화면 — 격자·오른쪽 패널·복사`

---

### Task 8: 전체 확인

- [ ] **Step 1:** `cargo fmt --check` · `cargo clippy --workspace -- -D warnings` · `cargo test --workspace` · `cd app && npm test && npx tsc --noEmit` 모두 통과.
- [ ] **Step 2:** `npm run tauri dev`(백그라운드, `CARGO_TERM_COLOR=never`)로 수동 확인:
  - 앱 시작·웹폰트 탭 진입만으로는 네트워크 요청이 없다.
  - Tabler [받기] → 격자, 선/채움, 검색, 패널 조절, 복사 6종(Figma에 SVG 붙여 넣기, PNG 4x 붙여 넣기 포함).
  - [세트 더 보기] → 193개 안팎 목록, Fluent UI System 받기 → 격자가 멈추지 않고 스크롤 시 더 불러옴.
  - [업데이트 확인] → "모두 최신입니다.", [삭제] → 다시 "받기 전".
  - 네트워크 끊고 앱 재시작 → 받은 세트 검색·복사 동작, [받기]는 "인터넷 연결이 필요합니다."
- [ ] **Step 3:** 결과(통과 항목·안 된 항목)를 보고. 문제가 있으면 해당 Task로 돌아가 고친 뒤 다시 확인.
