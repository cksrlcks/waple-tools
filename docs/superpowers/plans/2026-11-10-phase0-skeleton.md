# Phase 0 뼈대 + UI 시스템 이식 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cargo workspace + Tauri v2 앱을 만들고, udrive UI 시스템을 이식하고, 더미 도구(파일 복사)로 UI → Rust → 결과 흐름을 연결한다.

**Architecture:** `crates/core`가 Tool 계약(`out_dir`에만 쓰고 만든 파일 목록을 반환)을 정의하고, 도구는 크레이트 하나씩(`crates/tools/<카테고리>-<이름>`). `app/src-tauri`가 `builtin()` 목록으로 도구를 링크하고 `list_tools`·`run_job` 명령을 낸다. 프론트는 Vite + React + FSD, UI는 udrive `components/ui`를 `shared/ui`로 옮겨 쓴다. 배치 엔진(원본 이동·백업)은 Phase 1 — 이번엔 결과가 임시 폴더에 남는다.

**Tech Stack:** Rust 1.99 (edition 2024), Tauri 2, `serde`/`serde_json`/`schemars`/`thiserror`, React 19, Vite 8, TypeScript 6, Tailwind v4, `@base-ui/react`, TanStack Query 5, `@tauri-apps/plugin-dialog`

**Spec:** [docs/superpowers/specs/2026-11-10-image-first-addon-design.md](../specs/2026-11-10-image-first-addon-design.md) (+ [docs/plan.md](../../plan.md) 3장·7장·8장 Phase 0, `D:\project\git\udrive\docs\UI-SYSTEM.md` 5장)

## Global Constraints

- 화면 문구·오류 메시지·옵션 설명은 한국어.
- Tauri capability에 http·shell·원격 URL 권한 금지. 이번 Phase에는 네트워크 코드 없음.
- CSP: `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' data:; style-src 'self' 'unsafe-inline'`. 외부 폰트·CDN 금지 (Pretendard는 npm 패키지로 번들).
- 도구는 `out_dir` 밖에 쓰지 않는다. 원본 파일을 지우거나 덮어쓰지 않는다.
- 화면 코드(`pages`/`widgets`/`features`/`entities`)에 `#hex`, `bg-[…]`/`text-[…]`/`border-[…]`, `gray|slate|zinc|neutral|stone|blue|red|green-숫자` 금지. 토큰 클래스만.
- 컴포넌트 props는 `type`, 파일 하나 = 컴포넌트 하나, 파일명 kebab-case, named export.
- 창: 제목 `waple tools`, 최소 크기 1024×700. 번들 identifier `com.cksrlcks.waple-tools`.
- 끝날 때마다 `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` 통과.
- 프론트 의존성에 `@tailwindcss/typography` 넣지 않는다. Vitest는 이번 Phase에 넣지 않는다(그래서 `desktop-style.test.ts`도 복사 안 함).

## Review Focus

1. 경로에 한글·공백이 있는 파일 (`D:\내 사진\a b.png`) → 그대로 복사돼야 한다. → Task 2 테스트
2. 다른 폴더의 같은 이름 파일 두 개를 함께 실행 → 서로 덮어쓰지 않아야 한다 (파일마다 별도 `out_dir`). → Task 3 테스트
3. 입력 중 하나가 없는 파일 → 그 파일만 실패로 보고되고 나머지는 처리돼야 한다. → Task 3 테스트
4. 도구가 패닉 → 앱이 죽지 않고 그 파일이 실패로 보고돼야 한다. → Task 3 테스트
5. 실행 도중 취소 → 진행 중인 파일까지만 처리하고 나머지는 "취소됨"이어야 한다. → Task 3 테스트

---

## 파일 구조

```
waple-tools/
├─ .gitignore                       # target/, node_modules/, dist/
├─ Cargo.toml                       # workspace
├─ CLAUDE.md                        # docs/plan.md 9장 코드 블록 내용 그대로
├─ crates/
│  ├─ core/src/
│  │  ├─ lib.rs                     # pub mod + 재노출
│  │  ├─ tool.rs                    # Tool, ToolMeta, ToolKind, ToolOutput, Block
│  │  ├─ context.rs                 # JobContext (진행률·취소)
│  │  └─ error.rs                   # ToolError
│  └─ tools/dummy-copy/src/lib.rs   # 더미 도구 + 테스트
├─ app/
│  ├─ package.json, vite.config.ts, tsconfig.json, components.json, index.html, .prettierrc
│  ├─ src-tauri/
│  │  ├─ Cargo.toml, tauri.conf.json, capabilities/default.json
│  │  └─ src/
│  │     ├─ main.rs / lib.rs        # 앱 시작, 명령 등록
│  │     ├─ builtin.rs              # builtin() -> Vec<Box<dyn Tool>>
│  │     ├─ job.rs                  # run_files(): 파일별 out_dir, 패닉·취소 처리 (+ 테스트)
│  │     └─ commands.rs             # list_tools, run_job, cancel_job
│  └─ src/
│     ├─ main.tsx
│     ├─ app/
│     │  ├─ app.tsx                 # Providers + HomePage
│     │  ├─ providers.tsx           # QueryClientProvider
│     │  └─ styles/index.css        # udrive index.css (정리본)
│     ├─ pages/home/ui/home-page.tsx
│     ├─ widgets/ui-check/          # Phase 2에서 지울 임시 확인 패널
│     │  ├─ index.ts
│     │  └─ ui/ui-check-panel.tsx
│     ├─ features/tool-run/
│     │  ├─ index.ts
│     │  ├─ api/run-job.ts
│     │  └─ ui/tool-runner.tsx
│     ├─ entities/tool/
│     │  ├─ index.ts
│     │  ├─ model/types.ts
│     │  ├─ api/tool-keys.ts
│     │  ├─ api/use-tools.ts
│     │  └─ ui/tool-list.tsx
│     └─ shared/
│        ├─ ui/                     # udrive components/ui 10개
│        └─ lib/utils.ts
└─ docs/UI-SYSTEM.md                # udrive에서 복사
```

---

### Task 1: 저장소 + core 크레이트

**Files:**
- Create: `.gitignore`, `Cargo.toml`, `CLAUDE.md`, `crates/core/Cargo.toml`, `crates/core/src/{lib,tool,context,error}.rs`

**Interfaces:**
- Produces (모두 `waple_tools_core` 크레이트, `use waple_tools_core::*`로 접근):
  - `pub enum ToolKind { PerFile, Combine, Generator, Interactive }` — `Serialize`, `rename_all = "camelCase"`
  - `pub struct ToolMeta { pub id: String, pub name: String, pub category: String, pub description: String, pub extensions: Vec<String> }` — `Serialize`, camelCase. `extensions`는 소문자, 점 없음(`"png"`), 빈 목록 = 모든 확장자.
  - `pub enum Block { Text { text: String }, Code { lang: String, code: String }, Number { label: String, value: f64, unit: Option<String> }, Color { hex: String }, Image { path: PathBuf }, ImagePair { a: PathBuf, b: PathBuf, diff: PathBuf } }` — `Serialize`, `tag = "type"`, camelCase
  - `pub enum ToolOutput { Files(Vec<PathBuf>), Blocks(Vec<Block>) }` — `Files`는 `out_dir` 기준 상대 경로
  - `pub enum ToolError { Cancelled, InvalidOptions(String), Io(#[from] std::io::Error), Failed(String) }` — `thiserror`, `Display`는 한국어 (`"취소됨"`, `"옵션이 올바르지 않습니다: {0}"`, `"파일 오류: {0}"`, `"{0}"`)
  - `pub struct JobContext` — `pub fn new(on_progress: impl Fn(f32) + Send + Sync + 'static, cancel: Arc<AtomicBool>) -> Self`, `pub fn progress(&self, ratio: f32)` (0.0~1.0로 clamp 후 콜백), `pub fn is_cancelled(&self) -> bool`, `pub fn check_cancel(&self) -> Result<(), ToolError>` (취소면 `Err(Cancelled)`)
  - `pub trait Tool: Send + Sync { fn meta(&self) -> ToolMeta; fn kind(&self) -> ToolKind; fn options_schema(&self) -> serde_json::Value; fn run(&self, ctx: &JobContext, input: &[PathBuf], out_dir: &Path, opts: &serde_json::Value) -> Result<ToolOutput, ToolError>; }`

- [ ] **Step 1:** `git init`, `.gitignore`(`target/`, `node_modules/`, `dist/`, `app/src-tauri/gen/`), 루트 `Cargo.toml`:
  ```toml
  [workspace]
  resolver = "3"
  members = ["crates/core", "crates/tools/*", "app/src-tauri"]
  [workspace.package]
  edition = "2024"
  version = "0.1.0"
  [workspace.dependencies]
  serde = { version = "1", features = ["derive"] }
  serde_json = { version = "1", features = ["preserve_order"] }
  schemars = "1"
  thiserror = "2"
  tempfile = "3"
  ```
  `CLAUDE.md`는 [docs/plan.md](../../plan.md) 9장 코드 블록 내용을 그대로 옮긴다. (`app/src-tauri`는 Task 3에서 생기므로 이 Task에서는 members에서 잠시 빼 두고 Task 3에서 추가.)

- [ ] **Step 2: 실패하는 테스트 작성** — `crates/core/src/context.rs` 안 `#[cfg(test)]`
  ```rust
  #[test] fn progress_is_clamped() {
      let seen = Arc::new(Mutex::new(vec![]));
      let s = seen.clone();
      let ctx = JobContext::new(move |r| s.lock().unwrap().push(r), Arc::new(AtomicBool::new(false)));
      ctx.progress(-1.0); ctx.progress(0.5); ctx.progress(2.0);
      assert_eq!(*seen.lock().unwrap(), vec![0.0, 0.5, 1.0]);
  }
  #[test] fn check_cancel_reports_cancelled() {
      let flag = Arc::new(AtomicBool::new(false));
      let ctx = JobContext::new(|_| {}, flag.clone());
      assert!(ctx.check_cancel().is_ok());
      flag.store(true, Ordering::Relaxed);
      assert!(matches!(ctx.check_cancel(), Err(ToolError::Cancelled)));
  }
  ```
  `tool.rs`에 직렬화 테스트: `serde_json::to_value(Block::ImagePair{..})`의 `"type"`이 `"imagePair"`, `ToolKind::PerFile`이 `"perFile"`.

- [ ] **Step 3:** `cargo test -p waple-tools-core` → 컴파일 실패(타입 없음) 확인
- [ ] **Step 4:** Interfaces대로 구현
- [ ] **Step 5:** `cargo test -p waple-tools-core` PASS, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings` 통과
- [ ] **Step 6: Commit** — `git add . && git commit -m "chore: workspace와 core Tool 계약"`

### Task 2: 더미 도구 크레이트 `dummy-copy`

**Files:**
- Create: `crates/tools/dummy-copy/Cargo.toml` (패키지명 `waple-tools-tool-dummy-copy`), `crates/tools/dummy-copy/src/lib.rs`

**Interfaces:**
- Consumes: Task 1 전부
- Produces: `pub fn tool() -> Box<dyn waple_tools_core::Tool>` — meta `{ id: "dev.copy", name: "파일 복사(테스트)", category: "dev", description: "입력 파일을 그대로 복사합니다.", extensions: [] }`, kind `PerFile`, 옵션 스키마는 `schemars`로 만든 빈 옵션 구조체 `CopyOptions {}`.
- 동작: 입력마다 `ctx.check_cancel()?` → `out_dir/<파일명>`으로 복사 → `ctx.progress((i+1)/n)`. `ToolOutput::Files(vec![파일명])`.

- [ ] **Step 1: 실패하는 테스트** (`tempfile` dev-dependency)
  ```rust
  #[test] fn copies_into_out_dir_and_keeps_original() {
      let dir = tempfile::tempdir().unwrap();
      let src = dir.path().join("내 사진 a b.png");          // Review Focus 1
      std::fs::write(&src, b"abc").unwrap();
      let out = dir.path().join("out"); std::fs::create_dir(&out).unwrap();
      let ctx = JobContext::new(|_| {}, Default::default());
      let r = tool().run(&ctx, &[src.clone()], &out, &serde_json::json!({})).unwrap();
      assert!(matches!(r, ToolOutput::Files(ref f) if f == &[PathBuf::from("내 사진 a b.png")]));
      assert_eq!(std::fs::read(out.join("내 사진 a b.png")).unwrap(), b"abc");
      assert_eq!(std::fs::read(&src).unwrap(), b"abc");
  }
  #[test] fn cancelled_before_start_writes_nothing() { /* cancel=true → Err(Cancelled), out 폴더 비어 있음 */ }
  ```
- [ ] **Step 2:** `cargo test -p waple-tools-tool-dummy-copy` → FAIL 확인
- [ ] **Step 3:** 구현
- [ ] **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — `feat: 더미 파일 복사 도구`

### Task 3: Tauri 앱 뼈대 + `list_tools` / `run_job` / `cancel_job`

**Files:**
- Create: `app/` (create-tauri-app의 React+TS 템플릿), `app/src-tauri/src/{builtin,job,commands}.rs`
- Modify: 루트 `Cargo.toml` members에 `app/src-tauri` 추가, `app/src-tauri/tauri.conf.json`, `app/src-tauri/capabilities/default.json`, `app/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `waple_tools_core::*`, `waple_tools_tool_dummy_copy::tool`
- Produces (Rust):
  - `builtin.rs`: `pub fn builtin() -> Vec<Box<dyn Tool>>` — 지금은 `vec![waple_tools_tool_dummy_copy::tool()]`
  - `job.rs`:
    - `#[derive(Serialize)] #[serde(tag = "status", rename_all = "camelCase")] pub enum FileResult { Done { input: PathBuf, outputs: Vec<PathBuf> }, Failed { input: PathBuf, message: String }, Cancelled { input: PathBuf } }` — `outputs`는 절대 경로
    - `pub fn run_files(tool: &dyn Tool, inputs: &[PathBuf], job_dir: &Path, opts: &Value, cancel: Arc<AtomicBool>, on_progress: impl Fn(Progress) + Send + Sync + 'static) -> Vec<FileResult>`
    - `#[derive(Serialize, Clone)] #[serde(rename_all = "camelCase")] pub struct Progress { pub file_index: usize, pub total: usize, pub ratio: f32 }`
    - 규칙: 파일마다 `job_dir/<index>/`를 `out_dir`로 만든다 (Review Focus 2). 파일 시작 전 취소면 그 파일과 나머지는 `Cancelled`. 도구 호출은 `std::panic::catch_unwind(AssertUnwindSafe(..))`로 감싸 패닉 → `Failed { message: "도구 실행 중 오류가 발생했습니다." }` (Review Focus 4). 오류는 `ToolError`의 `Display`를 `message`로.
  - `commands.rs` (모두 `#[tauri::command]`):
    - `list_tools() -> Vec<ToolInfo>` — `ToolInfo { meta: ToolMeta, kind: ToolKind, options_schema: Value }` camelCase
    - `async run_job(state, tool_id: String, inputs: Vec<PathBuf>, opts: Value, on_progress: tauri::ipc::Channel<Progress>) -> Result<RunResult, String>` — `RunResult { job_id: String, files: Vec<FileResult> }`. `job_dir` = `std::env::temp_dir()/waple-tools/<job_id>`, `job_id`는 시각 기반 문자열(예: `20261110-153012-123`). `tauri::async_runtime::spawn_blocking`에서 `run_files`. 알 수 없는 `tool_id`면 `Err("도구를 찾을 수 없습니다.")`.
    - `cancel_job()` — 앱 상태의 `Arc<AtomicBool>`를 true로. (동시 작업은 하나 — `run_job` 시작 시 false로 리셋.) `// ponytail: 작업 하나만 취소 가능, 동시 작업은 Phase 1 배치 엔진에서 job_id별로`
- 설정:
  - `tauri.conf.json`: `productName: "waple tools"`, `identifier: "com.cksrlcks.waple-tools"`, 창 `title: "waple tools"`, `width: 1200, height: 800, minWidth: 1024, minHeight: 700`, `app.security.csp`는 Global Constraints 값, `build.devUrl: "http://localhost:1420"`
  - `capabilities/default.json`: `core:default`, `dialog:allow-open` 만
  - `tauri-plugin-dialog` 추가. 템플릿에 들어온 `opener` 플러그인과 `greet` 예제는 지운다.

- [ ] **Step 1: 앱 생성** — 루트에서 `npm create tauri-app@latest app -- --template react-ts --manager npm --identifier com.cksrlcks.waple-tools -y` (플래그가 안 먹으면 대화형으로 같은 값 선택). `app/src-tauri/Cargo.toml`에 `[package] name = "waple-tools"`, `edition.workspace = true`, `[lib] name = "waple_tools_lib"`, `waple-tools-core`·`waple-tools-tool-dummy-copy` path 의존성.
- [ ] **Step 2: 실패하는 테스트** — `job.rs`의 `#[cfg(test)]`, 테스트 전용 도구 `struct Panicky`(첫 입력에서 `panic!`), `struct Slow`(파일마다 cancel 확인 후 복사):
  - `same_name_files_do_not_collide` — `a/x.txt`("1"), `b/x.txt`("2")를 dummy로 실행 → 두 `Done`의 outputs 내용이 각각 "1", "2"
  - `missing_input_fails_only_that_file` — `[존재하는 파일, 없는 파일, 존재하는 파일]` → `[Done, Failed, Done]`, `Failed.message`가 비어 있지 않음
  - `panic_becomes_failed` — Panicky로 2개 → 첫째 `Failed { message: "도구 실행 중 오류가 발생했습니다." }`, 둘째도 결과가 있음(프로세스가 죽지 않음)
  - `cancel_stops_remaining_files` — `on_progress`에서 `file_index == 0` 진행률을 받으면 cancel=true → 결과 `[Done, Cancelled, Cancelled]`
- [ ] **Step 3:** `cargo test -p waple-tools` (src-tauri 패키지명) → FAIL 확인
- [ ] **Step 4:** `builtin.rs`, `job.rs`, `commands.rs` 구현, `lib.rs`에 `.plugin(tauri_plugin_dialog::init())`, `.manage(CancelFlag(Arc<AtomicBool>))`, `invoke_handler![list_tools, run_job, cancel_job]`
- [ ] **Step 5:** `cargo test --workspace` PASS, fmt, clippy 통과
- [ ] **Step 6: Commit** — `feat: Tauri 앱 뼈대와 list_tools·run_job 명령`

### Task 4: UI 시스템 이식

**Files:**
- Copy: `D:\project\git\udrive\src\index.css` → `app/src/app/styles/index.css`
- Copy: `D:\project\git\udrive\src\components\ui\{button,checkbox,context-menu,dialog,dropdown-menu,input,menu-styles,menubar,progress,tooltip}.*` → `app/src/shared/ui/`
- Copy: `D:\project\git\udrive\src\lib\utils.ts` → `app/src/shared/lib/utils.ts`
- Copy: `D:\project\git\udrive\docs\UI-SYSTEM.md` → `docs/UI-SYSTEM.md`
- Create: `app/components.json`, `app/.prettierrc`
- Modify: `app/vite.config.ts`, `app/tsconfig.json`, `app/package.json`, `app/src/main.tsx`, `app/index.html`
- Delete: 템플릿의 `app/src/App.tsx`, `App.css`, `assets/`, `public/` 예제 파일

**Interfaces:**
- Produces: `@/shared/ui/button`(`Button`, `buttonVariants`), `@/shared/ui/dialog`(`Dialog*`), `@/shared/ui/context-menu`(`ContextMenu*`), `@/shared/ui/progress`(`Progress*`), `@/shared/ui/tooltip`, `@/shared/ui/input`, `@/shared/ui/checkbox`, `@/shared/ui/dropdown-menu`, `@/shared/ui/menubar`, `@/shared/lib/utils`(`cn`). 별칭 `@/*` → `app/src/*`.

- [ ] **Step 1: 의존성** (app/에서)
  ```bash
  npm i tailwindcss @tailwindcss/vite tw-animate-css shadcn @base-ui/react class-variance-authority cn lucide-react @tabler/icons-react pretendard @tanstack/react-query @tauri-apps/plugin-dialog
  npm i -D @types/node prettier @ianvs/prettier-plugin-sort-imports prettier-plugin-tailwindcss
  ```
- [ ] **Step 2: 파일 복사 + 정리**
  - `index.css`: 5행 `@plugin "@tailwindcss/typography";`, `.prose-markdown` 블록 전체(245행~), `@utility bg-checker` 블록 삭제. 나머지(`:root`, `.dark`, `@theme inline`, `@custom-variant`)는 그대로.
  - `dialog.tsx`: `@/components/ui/button` → `@/shared/ui/button`. `menubar.tsx`: `@/components/ui/dropdown-menu` → `@/shared/ui/dropdown-menu`. `./menu-styles` 상대 경로는 그대로. 그 밖에 `@/components`·`@/lib` 경로가 남았는지 `grep -rn "@/components\|@/lib" app/src`로 확인 → 0건.
- [ ] **Step 3: 설정**
  - `components.json`: udrive 파일 기준, `tailwind.css: "src/app/styles/index.css"`, `aliases: { components: "@/shared/ui", utils: "@/shared/lib/utils", ui: "@/shared/ui", lib: "@/shared/lib", hooks: "@/shared/lib" }`
  - `vite.config.ts`: `plugins: [react(), tailwindcss()]`, `resolve.alias: { "@": path.resolve(__dirname, "./src") }`, 템플릿의 Tauri용 server 설정(포트 1420, strictPort, `src-tauri` watch 제외)은 유지
  - `tsconfig.json`: `"paths": { "@/*": ["./src/*"] }`
  - `main.tsx`: `import "./app/styles/index.css"` 후 `<App />`(`@/app/app`) 렌더. `navigator.userAgent`에 `Mac`이 있으면 `document.documentElement.dataset.os = "mac"` 는 넣지 않는다(Windows 전용).
  - `index.html`: `lang="ko"`, `<title>waple tools</title>`, 템플릿 vite.svg 링크 삭제
  - `.prettierrc`: `{ "semi": false, "plugins": ["@ianvs/prettier-plugin-sort-imports", "prettier-plugin-tailwindcss"], "tailwindFunctions": ["cn", "cva"], "importOrder": ["^react$", "<THIRD_PARTY_MODULES>", "", "^@/(.*)$", "", "^[./]"] }`
  - `app/src/app/app.tsx`는 이 Task에서 임시로 `<Button>확인</Button>` 하나만 렌더(Task 5에서 교체)
- [ ] **Step 4: 확인** — `npx tsc --noEmit` 오류 0, `npx vite build` 성공
- [ ] **Step 5: Commit** — `feat: udrive UI 시스템 이식 (shared/ui, 토큰 CSS, components.json)`

### Task 5: Phase 0 화면 — 도구 목록 · 더미 실행 · 진행률 · UI 확인

**Files:**
- Create: `app/src/app/{app,providers}.tsx`, `app/src/entities/tool/{index.ts,model/types.ts,api/tool-keys.ts,api/use-tools.ts,ui/tool-list.tsx}`, `app/src/features/tool-run/{index.ts,api/run-job.ts,ui/tool-runner.tsx}`, `app/src/widgets/ui-check/{index.ts,ui/ui-check-panel.tsx}`, `app/src/pages/home/ui/home-page.tsx`

**Interfaces:**
- Consumes: Task 3 명령(`list_tools`, `run_job`, `cancel_job`)과 직렬화 형태, Task 4의 `@/shared/ui/*`
- Produces (TS):
  - `entities/tool/model/types.ts`: `ToolKind = "perFile" | "combine" | "generator" | "interactive"`, `ToolMeta = { id; name; category; description; extensions: string[] }`, `ToolInfo = { meta: ToolMeta; kind: ToolKind; optionsSchema: unknown }`, `Progress = { fileIndex: number; total: number; ratio: number }`, `FileResult = { status: "done"; input: string; outputs: string[] } | { status: "failed"; input: string; message: string } | { status: "cancelled"; input: string }`, `RunResult = { jobId: string; files: FileResult[] }`
  - `entities/tool/api/tool-keys.ts`: `toolKeys = { all: ["tools"] as const, list: () => [...toolKeys.all, "list"] as const }`
  - `entities/tool/api/use-tools.ts`: `useTools()` — `useQuery({ queryKey: toolKeys.list(), queryFn: () => invoke<ToolInfo[]>("list_tools") })`
  - `entities/tool/ui/tool-list.tsx`: `ToolList({ tools, selectedId, onSelect, className })` — UI-SYSTEM.md 4장 사이드바 행 클래스(`my-px flex h-8 items-center gap-2 rounded-md px-2 text-sm font-medium`, 선택 `bg-selection`)
  - `features/tool-run/api/run-job.ts`: `runJob(toolId: string, inputs: string[], onProgress: (p: Progress) => void): Promise<RunResult>` — `new Channel<Progress>()`로 `invoke("run_job", { toolId, inputs, opts: {}, onProgress: channel })`; `cancelJob(): Promise<void>`
  - `features/tool-run/ui/tool-runner.tsx`: `ToolRunner({ tool, className })` — "파일 선택" 버튼(`@tauri-apps/plugin-dialog`의 `open({ multiple: true })`), 선택 파일 목록, "실행"·"취소" 버튼, `Progress` 막대(전체 비율 = `(fileIndex + ratio) / total`), 결과 목록(완료는 출력 경로, 실패는 `text-destructive` 메시지, 취소는 `text-muted-foreground` "취소됨"). `invoke` 실패는 오류 배너(UI-SYSTEM.md 4장 오류 배너 클래스)로 표시하고 화면은 유지.
  - `widgets/ui-check/ui/ui-check-panel.tsx`: `UiCheckPanel({ className })` — 다크 전환 버튼(`document.documentElement.classList.toggle("dark")`), Button 6개 variant, Dialog 열기 버튼(UI-SYSTEM.md 3장 예시 그대로, 제목 "확인", 설명 "다이얼로그 표시 확인", 취소/확인), 우클릭 영역 + ContextMenu(항목 "열기", "이름 바꾸기", 구분선, "삭제" `variant="destructive"`). 파일 맨 위에 `// Phase 2에서 삭제: UI 이식 확인용` 주석.
  - `pages/home/ui/home-page.tsx`: `HomePage()` — UI-SYSTEM.md 4장 창 레이아웃 축소판: 바깥 `flex h-screen flex-col overflow-hidden bg-background text-sm text-foreground`, 왼쪽 사이드바(`w-60 border-r bg-sidebar p-2`)에 `ToolList`, 메인에 선택한 도구의 `ToolRunner`, 그 아래 `UiCheckPanel`. 로딩은 `text-muted-foreground` "불러오는 중…", 오류는 오류 배너.

- [ ] **Step 1:** 위 파일 구현 (`app.tsx` = `<Providers><HomePage /></Providers>`, `providers.tsx` = `QueryClientProvider` + `new QueryClient({ defaultOptions: { queries: { retry: false } } })`)
- [ ] **Step 2: 정적 확인** — `npx tsc --noEmit` 0건, `npx vite build` 성공, `npx prettier --check src` 통과, 화면 코드 금지 색 검사: `grep -rnE "#[0-9a-fA-F]{3,6}|(bg|text|border)-\[|(gray|slate|zinc|neutral|stone|blue|red|green)-[0-9]" app/src/{pages,widgets,features,entities}` → 0건
- [ ] **Step 3: 앱 흐름 확인** — `npm run tauri dev` → 사이드바에 "파일 복사(테스트)" 표시 → 선택 → 파일 2개 선택 → 실행 → 진행률 막대가 100%까지 → 결과에 `%TEMP%\waple-tools\<job_id>\0\…`, `…\1\…` 경로 표시, 원본 파일은 그대로
- [ ] **Step 4: UI 확인** — `npx vite`(브라우저, Tauri 없이) 열고 playwright-cli로 Button 줄, Dialog 열린 상태, ContextMenu 열린 상태를 라이트 / 다크 전환 후 각각 스크린샷. 확인: 버튼 26px·파랑 primary, 다이얼로그 아래 회색 띠, 메뉴 반투명 + 강조 시 파랑, 다크에서 글자 대비 정상. 브라우저에서는 `list_tools`가 실패하므로 오류 배너가 뜨고 나머지 화면은 그대로여야 함 (Review Focus 외 확인 항목).
- [ ] **Step 5:** `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` 최종 통과
- [ ] **Step 6: Commit** — `feat: Phase 0 화면 (도구 목록, 더미 실행, UI 확인 패널)`

---

## Phase 0 완료 기준 대조 (plan.md 8장)

| 기준 | 확인 위치 |
| --- | --- |
| `cargo test` 통과 | Task 5 Step 5 |
| 앱 실행 → 도구 목록 → 더미 실행 → 진행률 | Task 5 Step 3 |
| `tsc`·`vite build` | Task 4 Step 4, Task 5 Step 2 |
| Button·Dialog·ContextMenu 라이트/다크 | Task 5 Step 4 |
| UI-SYSTEM.md 기준 문서 | Task 4 (`docs/UI-SYSTEM.md`) |
