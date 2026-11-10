# Phase 1 배치 엔진 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 출력 정책 3종, 폴더 순회, 원본 백업, 용량 증가 시 원본 유지, 작업 로그, 되돌리기, dry-run, 병렬·취소를 갖춘 배치 엔진을 `crates/core`에 만들고, CLI와 Tauri 명령을 이 엔진으로 연결한다.

**Architecture:** 도구는 지금처럼 임시 폴더(`out_dir`)에만 쓴다. 엔진이 ① 입력을 모아 계획(dry-run)을 만들고 ② 파일마다 결과 폴더 옆에 `.waple-tools-tmp-…` 임시 폴더를 만들어 도구를 병렬 실행하고 ③ 성공한 파일만 전역 잠금 안에서 "원본 → 백업, 결과 → 최종 이름"으로 옮기며 모든 이동·생성을 JSONL 로그에 즉시 남긴다. 되돌리기는 로그를 역순으로 적용한다. 기본 내장 목록 `builtin()`은 CLI와 앱이 함께 쓰도록 `crates/builtin`으로 옮긴다.

**Tech Stack:** Rust 2024, `rayon`(MIT/Apache-2.0), `sha2`(MIT/Apache-2.0), `serde_json`, Tauri 2

**Spec:** [docs/plan.md](../../plan.md) 3.4 출력 정책 · 3.5 배치 엔진 1~12 · 8장 Phase 1, [설계 문서](../specs/2026-11-10-image-first-addon-design.md) 6.3(도구는 out_dir에만 쓰고 이동은 엔진이)

## Global Constraints

- 어떤 경우에도 기존 파일을 덮어쓰지 않는다. 이름이 겹치면 `name (1).ext`, `name (2).ext` … (plan.md 3.5-4)
- Windows의 `std::fs::rename`은 대상이 있으면 덮어쓴다 → 이동은 반드시 `fsops::move_new`(대상 없음 확인 + 전역 잠금)로만 한다.
- 원본은 성공했을 때만 옮긴다. 실패·건너뜀·취소된 파일의 원본은 그대로, 임시 폴더는 남기지 않는다.
- 순회 제외: `original` 폴더, 숨김 폴더(이름이 `.`로 시작 또는 Windows 숨김 속성), `.waple-tools-tmp-`로 시작하는 폴더, 도구가 지원하지 않는 확장자. (plan.md 3.5-1)
- "결과가 원본보다 크면 원본 유지" 기본 켜짐 → 결과 파일이 1개이고 원본보다 크면 `Skipped { reason: "용량 증가" }`. (3.5-5)
- 동시 작업 수 기본값: CPU 코어 수 − 1 (최소 1). (3.5-11)
- 사용자에게 보이는 문구는 한국어.
- 의존성을 추가하는 커밋 메시지에 이유와 라이선스를 적는다 (CLAUDE.md).
- 끝날 때마다 `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` 통과.

## 이번 계획에서 정한 것 (plan.md와 다른 점)

- **로그 위치:** plan.md 3.5-7은 `<작업 루트>/original/.waple-tools-log/`이지만, 그러면 원본을 그대로 두는 `KeepOriginal`·`OutputFolder`에서도 `original/` 폴더가 생긴다. 로그는 앱 데이터 폴더 `%LOCALAPPDATA%\waple tools\logs\<job_id>.jsonl`에 둔다 (엔진은 `log_dir` 인자로 받음). 로그 보관 기간 설정(plan.md 7장)도 한 곳에서 관리된다.
- **로그 형식:** 한 줄 = 한 동작인 JSONL. 이동할 때마다 바로 한 줄 쓰므로 중간에 앱이 꺼져도 그때까지는 되돌릴 수 있다.
- **`builtin()` 위치:** CLI도 써야 하므로 `app/src-tauri/src/builtin.rs` → `crates/builtin`. 내장↔추가 전환 시 고치는 곳이 이 크레이트 하나가 된다 (설계 6.4의 "src-tauri의 builtin()"을 이것으로 읽는다).
- **범위:** `PerFile` 도구만. `Combine`·`Generator` 결과 배치는 그런 도구가 생기는 Phase 5에서 (`run`에 넣으면 `Failed { "지원하지 않는 도구 종류입니다." }`).
- **진행률:** 병렬이라 파일 하나 안의 비율 대신 `Progress { done, total }` (파일 하나 끝날 때마다).

## Review Focus

1. 결과 파일 이름이 이미 다른 파일로 있을 때 (`a.png` 변환 결과 `a.webp`인데 `a.webp`가 이미 있음) → 기존 파일은 그대로, 결과는 `a (1).webp`. → Task 4 `in_place_output_name_collision_gets_number`
2. 도구가 `out_dir` 밖을 가리키는 경로(`..\x`, 절대 경로)를 돌려줄 때 → 그 파일은 실패, 원본 그대로. → Task 4 `escaping_output_path_fails`
3. 되돌리기 전에 사용자가 결과 파일을 고쳤을 때 → 그 파일은 지우지 않고 "건너뜀(수정됨)"으로 알림, 나머지는 복구. → Task 6 `undo_skips_modified_result`
4. 같은 작업을 두 번 되돌리기 → 두 번째는 "이미 되돌린 작업입니다." 오류, 아무것도 바꾸지 않음. → Task 6 `undo_twice_is_rejected`
5. 처리 도중 도구가 패닉 → 그 파일만 실패, 원본·다른 파일 정상, 임시 폴더 없음. → Task 4 `failing_and_panicking_files_keep_originals`

---

## 파일 구조

```
crates/core/src/
├─ lib.rs                 # pub mod batch 추가
└─ batch/
   ├─ mod.rs              # 공개 타입(OutputPolicy, JobSpec, FileResult, JobReport, Progress …) + pub use
   ├─ fsops.rs            # unique_path, move_new, file_hash, 전역 COMMIT_LOCK
   ├─ collect.rs          # 입력 모으기 + plan() (dry-run)
   ├─ log.rs              # JobLog(JSONL append), read_log, LogEntry
   ├─ run.rs              # run(): 병렬 실행·크기 비교·커밋·로그
   └─ undo.rs             # undo()
crates/core/tests/batch.rs        # 임시 폴더 통합 테스트 (도구 대역 포함)
crates/builtin/src/lib.rs         # builtin(), find(id), default_log_dir(), default_threads()
crates/cli/src/main.rs            # waple-tools run / undo / preview
crates/tools/dev-copy/            # dummy-copy 이름 변경
app/src-tauri/src/commands.rs     # 엔진 연결, preview_job, undo_job, job별 취소
app/src-tauri/src/job.rs          # 삭제 (엔진으로 대체)
app/src/entities/tool/model/types.ts, features/tool-run/*  # 새 결과·진행률 형태
```

---

### Task 1: 정리 — 더미 도구 이름, tsbuildinfo

**Files:**
- Rename: `crates/tools/dummy-copy/` → `crates/tools/dev-copy/` (패키지명 `waple-tools-tool-dev-copy`, 라이브러리 경로 참조 전부)
- Modify: `app/src-tauri/Cargo.toml`, `app/src-tauri/src/{builtin,job}.rs`의 `waple_tools_tool_dummy_copy` → `waple_tools_tool_dev_copy`
- Modify: `app/.gitignore`에 `*.tsbuildinfo` 추가, `git rm --cached app/tsconfig.node.tsbuildinfo`

- [ ] **Step 1:** 이름 변경 후 `cargo test --workspace` → 기존 11개 PASS, `git ls-files app | grep tsbuildinfo` → 0건
- [ ] **Step 2: Commit** — `chore: 더미 도구 크레이트를 dev-copy로 이름 변경, tsbuildinfo 추적 해제`

### Task 2: 파일 이동 기본기 (`batch/fsops.rs`)

**Files:** Create `crates/core/src/batch/{mod,fsops}.rs`, Modify `crates/core/src/lib.rs`, `crates/core/Cargo.toml`(+`sha2 = "0.10"`, dev `tempfile`)

**Interfaces — Produces** (`waple_tools_core::batch::fsops`, `pub(crate)` 아님 — 테스트·undo가 씀):
- `pub fn unique_path(want: &Path) -> PathBuf` — `want`가 없으면 그대로, 있으면 `stem (1).ext`, `stem (2).ext` … 처음 비어 있는 것. 확장자 없으면 `stem (1)`.
- `pub fn move_new(from: &Path, want: &Path) -> io::Result<PathBuf>` — `COMMIT_LOCK`을 잡고 부모 폴더 생성 → `unique_path(want)` → `rename`, rename이 실패하면(다른 드라이브) `copy` 후 `remove_file(from)`. 실제로 옮긴 경로 반환.
- `pub fn file_hash(p: &Path) -> io::Result<String>` — sha256 소문자 hex.
- `static COMMIT_LOCK: Mutex<()>` (`// ponytail: 전역 잠금 — 커밋(이동)만 직렬화, 도구 실행은 병렬. 이동은 ms 단위라 병목 아님`)

- [ ] **Step 1: 실패하는 테스트** (`fsops.rs` 안 `#[cfg(test)]`)
  - `unique_path_numbers_until_free`: `a.txt`, `a (1).txt` 존재 → `a (2).txt`; `README`(확장자 없음) 존재 → `README (1)`
  - `move_new_never_overwrites`: `src.txt`("new"), `dst.txt`("old") → 결과 경로 `dst (1).txt`, `dst.txt` 내용 "old" 그대로, `src.txt` 없음
  - `move_new_creates_parent_dirs`: 대상 `x/y/z.txt` → 성공
  - `file_hash_is_sha256`: "abc" → `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`
- [ ] **Step 2:** `cargo test -p waple-tools-core fsops` → 컴파일 실패 확인
- [ ] **Step 3:** 구현
- [ ] **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — 본문에 `sha2 0.10 추가: 되돌리기 시 결과 파일이 수정됐는지 확인 (MIT/Apache-2.0)`

### Task 3: 입력 모으기와 dry-run (`batch/collect.rs`, 공개 타입)

**Interfaces — Produces** (`waple_tools_core::batch`, 모두 `Serialize` camelCase, `Deserialize`가 필요한 건 명시):
- `#[serde(tag = "type")] pub enum OutputPolicy { InPlaceWithBackup, KeepOriginal { suffix: String }, OutputFolder { dir: PathBuf } }` — `Deserialize`, `Default` = `InPlaceWithBackup`
- `pub struct JobSpec { pub inputs: Vec<PathBuf>, pub recursive: bool, pub policy: OutputPolicy, pub keep_if_larger: bool, pub threads: usize, pub opts: serde_json::Value }` — `Deserialize`. `keep_if_larger` 기본 true, `threads` 0이면 기본값(코어−1, 최소 1).
- `pub struct PlannedFile { pub input: PathBuf, pub root: PathBuf, pub rel: PathBuf, pub dest_dir: PathBuf }`
  - 폴더 입력: `root` = 그 폴더, `rel` = root 기준 상대 경로. 파일 입력: `root` = 파일의 부모 폴더, `rel` = 파일 이름.
  - `dest_dir`: InPlace·Keep → 입력 파일의 폴더, OutputFolder → `dir.join(rel.parent())`
- `pub struct Skipped { pub path: PathBuf, pub reason: String }`
- `pub struct Plan { pub files: Vec<PlannedFile>, pub skipped: Vec<Skipped> }`
- `pub fn plan(tool: &dyn Tool, spec: &JobSpec) -> Plan` — 파일 시스템을 바꾸지 않는다.
  - 확장자 비교는 소문자. `extensions`가 비면 모두 허용.
  - 지원 안 하는 확장자 → `skipped { reason: "지원하지 않는 형식" }`, 없는 경로 → `"파일을 찾을 수 없음"`. 제외 폴더(original·숨김·tmp)는 조용히 건너뛰고 skipped에 넣지 않는다.
  - 같은 파일이 두 번 들어오면(폴더 + 그 안의 파일) 한 번만.
  - 하위 폴더는 `recursive`일 때만. 순회는 `std::fs::read_dir` 재귀 (`walkdir` 안 씀 — 몇 줄이면 됨).

- [ ] **Step 1: 실패하는 테스트** — `crates/core/tests/batch.rs` (도구 대역 `Ext(&["png"])`: extensions만 다른 복사 도구; 테스트 파일 상단 `mod support`에 `CopyTool { exts, grow: bool, fail_on: Option<&str>, panic_on: Option<&str>, rename_to_ext: Option<&str>, escape: bool }` 하나로 모든 대역을 만든다)
  - `plan_excludes_original_hidden_tmp_and_unsupported`: 폴더에 `a.png`, `b.txt`, `original/c.png`, `.git/d.png`, `.waple-tools-tmp-1/e.png`, `sub/f.png` → recursive=false: files=[a.png], skipped=[b.txt "지원하지 않는 형식"]; recursive=true: files=[a.png, sub/f.png] (rel `sub/f.png`)
  - `plan_single_file_root_is_parent`: 입력 `dir/a.png` → root `dir`, rel `a.png`
  - `plan_output_folder_keeps_structure`: OutputFolder{out} + recursive → `sub/f.png`의 dest_dir = `out/sub`
  - `plan_changes_nothing`: plan 전후 폴더 트리(경로+내용) 동일
  - `plan_dedups_same_file`: 입력 [dir, dir/a.png] → files 1개
- [ ] **Step 2:** `cargo test -p waple-tools-core --test batch plan_` → FAIL 확인
- [ ] **Step 3:** 구현 (Windows 숨김 속성은 `#[cfg(windows)] MetadataExt::file_attributes() & 0x2`)
- [ ] **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — `feat: 배치 입력 모으기와 dry-run`

### Task 4: 실행 엔진 — InPlaceWithBackup + 로그 (`batch/run.rs`, `batch/log.rs`)

**Interfaces — Produces:**
- `log.rs`:
  - `#[serde(tag = "op", rename_all = "camelCase")] pub enum LogEntry { Moved { from: PathBuf, to: PathBuf, hash: String }, Created { path: PathBuf, hash: String } }` — `Moved`는 원본을 백업으로 옮긴 것(`hash`는 옮긴 뒤 `to`의 해시), `Created`는 결과 파일.
  - `pub struct JobLog` — `pub fn create(log_dir: &Path, job_id: &str) -> io::Result<Self>`(파일 새로 만듦, 이미 있으면 오류), `pub fn append(&self, e: &LogEntry) -> io::Result<()>`(한 줄 쓰고 flush, 내부 `Mutex<File>`)
  - `pub fn read_log(log_dir: &Path, job_id: &str) -> io::Result<Vec<LogEntry>>`
- `mod.rs`:
  - `#[serde(tag = "status", rename_all = "camelCase")] pub enum FileResult { Done { input, outputs: Vec<PathBuf>, backup: Option<PathBuf>, size_before: u64, size_after: u64 }, Skipped { input, reason: String }, Failed { input, message: String }, Cancelled { input } }`
  - `pub struct Progress { pub done: usize, pub total: usize }`
  - `pub struct JobReport { pub job_id: String, pub files: Vec<FileResult>, pub skipped: Vec<Skipped> }` — `skipped`는 plan 단계 제외 목록
- `run.rs`: `pub fn run(tool: &dyn Tool, spec: &JobSpec, job_id: &str, log_dir: &Path, cancel: Arc<AtomicBool>, on_progress: impl Fn(Progress) + Send + Sync) -> io::Result<JobReport>` — `io::Result`는 로그 파일을 못 만들 때만.
- 파일 하나 처리 순서 (InPlaceWithBackup):
  1. 취소면 `Cancelled`.
  2. `tmp = dest_dir/.waple-tools-tmp-<job_id>-<index>/` 생성 → `catch_unwind`로 `tool.run(ctx, [input], tmp, opts)`. 패닉이면 payload(`&str`/`String`)를 `eprintln!("[waple-tools] {tool_id} 패닉: {msg}")`로 남기고 `Failed { "도구 실행 중 오류가 발생했습니다." }`.
  3. 결과 경로 검증: 상대 경로이고 `Component::Normal`만 → 아니면 `Failed { "도구가 잘못된 결과 경로를 돌려주었습니다." }` (Review Focus 2).
  4. `keep_if_larger`이고 결과가 1개이고 결과 크기 > 원본 크기 → `Skipped { "용량 증가" }`.
  5. 커밋: `backup = move_new(input, root/original/rel)` → 로그 `Moved` → 결과마다 `move_new(tmp/f, dest_dir/f)` → 로그 `Created`. 결과 이동 중 실패하면 이미 옮긴 결과를 지우고 원본을 `input` 자리로 되돌린 뒤 `Failed`.
  6. 어떤 경우든 마지막에 `tmp` 폴더 삭제 (`remove_dir_all`).
- 병렬: `rayon::ThreadPoolBuilder::new().num_threads(threads).build()` + `par_iter`, 결과는 입력 순서대로. `Progress.done`은 `AtomicUsize`.
- `ToolOutput::Blocks` → `Failed { "지원하지 않는 도구 종류입니다." }`, `tool.kind() != PerFile`도 같은 문구.

- [ ] **Step 1: 실패하는 테스트** (`tests/batch.rs`)
  - `in_place_backs_up_and_replaces`: `root/a.png`("A"), `root/sub/b.png`("B"), recursive → `root/a.png`=="A"(결과), `root/original/a.png`=="A", `root/original/sub/b.png`=="B", `.waple-tools-tmp-*` 없음, 로그에 Moved 2 + Created 2
  - `backup_name_collision_gets_number`: `root/original/a.png`("old") 미리 있음 → 백업은 `original/a (1).png`, "old" 그대로
  - `larger_result_keeps_original`: `grow: true` 도구 → `Skipped { reason: "용량 증가" }`, `original/` 없음, 원본 그대로, tmp 없음
  - `failing_and_panicking_files_keep_originals`: 입력 a,b,c, `fail_on: "a"`, `panic_on: "b"` → [Failed, Failed("도구 실행 중 오류가 발생했습니다."), Done], a·b 원본 그대로, tmp 없음 (Review Focus 5)
  - `extension_change_moves_only_original`: `rename_to_ext: "webp"` → `root/a.webp` 생성, `root/a.png` 없음, `original/a.png` 있음
  - `in_place_output_name_collision_gets_number`: `root/a.webp`("keep") 미리 있음 + `rename_to_ext: "webp"` → 결과 `root/a (1).webp`, `a.webp`=="keep" (Review Focus 1)
  - `escaping_output_path_fails`: `escape: true`(결과로 `../x.png` 반환) → Failed, 원본 그대로 (Review Focus 2)
- [ ] **Step 2:** `cargo test -p waple-tools-core --test batch` → 새 테스트 FAIL 확인
- [ ] **Step 3:** 구현 (`rayon = "1"` 추가)
- [ ] **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — 본문에 `rayon 1 추가: 파일 병렬 처리 (MIT/Apache-2.0)`

### Task 5: KeepOriginal · OutputFolder

**Interfaces:** Task 4 `run`의 5번 커밋 단계만 정책별로 분기.
- KeepOriginal{suffix}: 원본은 안 옮김. 결과 `f`의 최종 이름 = `{f의 stem}{suffix}.{f의 ext}` → `move_new(tmp/f, dest_dir/그 이름)` → `Created`. `backup: None`.
- OutputFolder{dir}: 원본은 안 옮김. `move_new(tmp/f, dest_dir/f)` (dest_dir는 plan이 정함) → `Created`. `backup: None`.
- 두 정책 모두 `keep_if_larger` 비교는 똑같이 적용.

- [ ] **Step 1: 실패하는 테스트**
  - `keep_original_adds_suffix`: suffix `_copy` → `root/a_copy.png` 생성, `root/a.png` 그대로, `original/` 없음; 다시 한 번 실행하면 `a_copy (1).png`
  - `output_folder_keeps_structure_and_originals`: recursive + OutputFolder{out} → `out/a.png`, `out/sub/b.png`, 원본 둘 다 그대로
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — `feat: KeepOriginal·OutputFolder 출력 정책`

### Task 6: 취소와 되돌리기 (`batch/undo.rs`)

**Interfaces — Produces:**
- `#[serde(tag = "status", rename_all = "camelCase")] pub enum UndoItem { Restored { path: PathBuf }, Removed { path: PathBuf }, Skipped { path: PathBuf, reason: String } }`
- `pub fn undo(log_dir: &Path, job_id: &str) -> Result<Vec<UndoItem>, UndoError>`
- `#[derive(thiserror::Error)] pub enum UndoError { #[error("작업 기록을 찾을 수 없습니다.")] NotFound, #[error("이미 되돌린 작업입니다.")] AlreadyUndone, #[error("파일 오류: {0}")] Io(#[from] io::Error) }`
- 동작: 로그를 역순으로
  - `Created { path, hash }`: 있고 해시 같으면 삭제 → `Removed`. 없으면 `Skipped "이미 없음"`, 해시 다르면 `Skipped "수정됨"`.
  - `Moved { from, to, hash }`: `to` 해시 같고 `from`이 비어 있으면 `move_new(to, from)` → `Restored`. `from`이 이미 있으면 `Skipped "원래 자리에 다른 파일이 있음"`, 해시 다르면 `Skipped "수정됨"`.
  - 복원 후 `to`의 부모 폴더들을 `original` 폴더까지 거슬러 올라가며 `remove_dir`(비어 있을 때만 지워짐).
  - 끝나면 로그 파일을 `<job_id>.undone.jsonl`로 이름 변경. 그 파일이 이미 있으면 `AlreadyUndone`.

- [ ] **Step 1: 실패하는 테스트**
  - `undo_restores_tree_exactly`: 트리(`a.png`, `sub/b.png`, `memo.txt`) 스냅샷 → run(InPlace, recursive) → undo → 스냅샷 동일 (`original/` 폴더 없음)
  - `undo_keep_original_removes_results`: KeepOriginal 실행 후 undo → 스냅샷 동일
  - `cancel_then_undo`: 입력 3개, threads=1, `on_progress`에서 `done == 1`이면 cancel → 결과 [Done, Cancelled, Cancelled], 원본 b·c 그대로; undo → 스냅샷 동일
  - `undo_skips_modified_result`: run 후 결과 `root/a.png`를 "edited"로 수정 → undo 결과에 `Skipped { reason: "수정됨" }`, `root/a.png`=="edited" 유지 (Review Focus 3)
  - `undo_twice_is_rejected`: 두 번째 undo → `Err(AlreadyUndone)`, 트리 변화 없음 (Review Focus 4)
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: Commit** — `feat: 로그 기반 되돌리기`

### Task 7: `crates/builtin` + CLI

**Files:** Create `crates/builtin/{Cargo.toml,src/lib.rs}`(패키지 `waple-tools-builtin`), `crates/cli/{Cargo.toml,src/main.rs}`(패키지 `waple-tools-cli`, bin 이름 `waple-tools`). Delete `app/src-tauri/src/builtin.rs`.

**Interfaces — Produces** (`waple_tools_builtin`):
- `pub fn builtin() -> Vec<Box<dyn Tool>>` — `vec![waple_tools_tool_dev_copy::tool()]`
- `pub fn find(id: &str) -> Option<Box<dyn Tool>>`
- `pub fn default_log_dir() -> PathBuf` — `%LOCALAPPDATA%\waple tools\logs` (`LOCALAPPDATA` 없으면 `std::env::temp_dir()/waple tools/logs`)
- `pub fn new_job_id() -> String` — `YYYYMMDD-HHMMSS-mmm` (UTC, std만 사용해 계산) + `-` + 4자리 난수 대신 프로세스 내 카운터(`AtomicU32`) → 같은 밀리초 충돌 방지

**CLI** (인자 파싱은 std만):
```
waple-tools run <tool-id> <경로>... [--opts '<json>'] [--recursive] [--keep <suffix> | --out <dir>] [--dry-run]
waple-tools undo <job_id>
```
- `run`: `--dry-run`이면 `plan` 결과를, 아니면 `run` 결과를 JSON(pretty)으로 stdout. 끝에 `job_id` 출력. 진행률은 stderr에 `done/total`.
- 잘못된 인자 → stderr에 사용법, 종료 코드 2. 엔진 오류 → 메시지, 종료 코드 1.
- 파싱은 `fn parse(args: &[String]) -> Result<Command, String>`로 분리.

- [ ] **Step 1: 실패하는 테스트** (`main.rs` `#[cfg(test)]`)
  - `parse_run_with_flags`: `["run","dev.copy","a","b","--recursive","--keep","_x","--opts","{\"q\":1}"]` → Run{tool "dev.copy", inputs [a,b], recursive true, policy KeepOriginal{"_x"}, opts {"q":1}, dry_run false}
  - `parse_rejects_keep_and_out_together`: `--keep _x --out d` → Err
  - `parse_undo`: `["undo","20261110-010203-004-1"]` → Undo{job_id}
  - (`waple_tools_builtin`) `job_ids_are_unique`: 1000번 호출 → 모두 다름
- [ ] **Step 2:** FAIL 확인 → **Step 3:** 구현 → **Step 4:** PASS + fmt + clippy
- [ ] **Step 5: 수동 확인** — 임시 폴더에 파일 2개 → `cargo run -p waple-tools-cli -- run dev.copy <폴더>` → `original/` 생김 → `cargo run -p waple-tools-cli -- undo <job_id>` → 원상 복구
- [ ] **Step 6: Commit** — `feat: builtin 크레이트와 CLI(run/undo/--dry-run)`

### Task 8: Tauri 명령을 엔진으로 연결

**Files:** Modify `app/src-tauri/src/{lib,commands}.rs`, `app/src-tauri/Cargo.toml`, Delete `app/src-tauri/src/job.rs`; Modify `app/src/entities/tool/model/types.ts`, `app/src/entities/tool/index.ts`, `app/src/features/tool-run/{api/run-job.ts,ui/tool-runner.tsx,ui/file-result-row.tsx}`, `app/src/shared/lib/to-user-message.ts`

**Interfaces — Produces:**
- Rust 명령:
  - `list_tools()` — 그대로 (`waple_tools_builtin::builtin()` 사용)
  - `preview_job(tool_id: String, spec: JobSpec) -> Result<Plan, String>`
  - `new_job_id() -> String` — `waple_tools_builtin::new_job_id()`. 프론트가 실행 전에 받아 두어야 실행 중 취소 버튼이 job_id를 안다.
  - `async run_job(jobs: State<Jobs>, job_id: String, tool_id: String, spec: JobSpec, on_progress: Channel<Progress>) -> Result<JobReport, String>` — `Jobs`(= `Mutex<HashMap<String, Arc<AtomicBool>>>`)에 등록 후 `spawn_blocking`으로 실행, 끝나면 제거. 같은 job_id가 이미 있으면 `Err("이미 실행 중인 작업입니다.")`.
  - `cancel_job(jobs, job_id: String)` — 해당 플래그만 true
  - `undo_job(job_id: String) -> Result<Vec<UndoItem>, String>`
- TS 타입: `Progress = { done: number; total: number }`, `FileResult`는 Task 4 형태(`done`에 `backup: string | null; sizeBefore; sizeAfter`, `skipped { reason }` 추가), `RunResult` → `JobReport = { jobId; files: FileResult[]; skipped: { path: string; reason: string }[] }`, `OutputPolicy`, `JobSpec`(camelCase: `inputs, recursive, policy, keepIfLarger, threads, opts`).
- `tool-runner.tsx`: `spec = { inputs, recursive: false, policy: { type: "InPlaceWithBackup" }, keepIfLarger: true, threads: 0, opts: {} }`로 실행 (정책 선택 UI는 Phase 2). 진행률 `done/total`. 결과 아래 "되돌리기" 버튼 → `undo_job` → 결과 문구 "되돌렸습니다 (복구 n, 건너뜀 m)". 파일 선택·취소·되돌리기 실패는 모두 `ErrorBanner`로 (Phase 0 리뷰 Minor 3).
- `toUserMessage`: 한글이 들어 있는 문자열만 그대로, 그 밖(Tauri 내부 영어 오류 등)은 `"작업을 실행할 수 없습니다."` (Phase 0 리뷰 Minor 4) — `/[가-힣]/` 검사.
- `OutputPolicy`의 serde 태그 값은 Rust variant 이름 그대로(`"InPlaceWithBackup"` 등) — `rename_all` 없이 `tag = "type"`.

- [ ] **Step 1:** Rust: `cargo test --workspace` PASS (job.rs 테스트는 Task 4 테스트로 대체됨), fmt, clippy
- [ ] **Step 2:** 프론트: `npx tsc --noEmit`, `npx vite build`, `npx prettier --check src`, 금지 색 grep 0건
- [ ] **Step 3: 수동 확인** — `npm run tauri dev` → 폴더 안 파일 2개 선택 → 실행 → 원래 자리에 결과, `original/`에 원본 → 되돌리기 → 원상 복구
- [ ] **Step 4: Commit** — `feat: 앱 실행·미리보기·되돌리기를 배치 엔진으로 연결`

---

## Phase 1 완료 기준 대조 (plan.md 8장)

| 기준 | 테스트 |
| --- | --- |
| 처리/백업 | `in_place_backs_up_and_replaces` |
| 이름 충돌 | `backup_name_collision_gets_number`, `in_place_output_name_collision_gets_number` |
| 용량 증가 시 원본 유지 | `larger_result_keeps_original` |
| 중간 실패 시 원본 보존 | `failing_and_panicking_files_keep_originals` |
| 취소 후 되돌리기 | `cancel_then_undo` |
| 되돌리기 후 원상 복구 | `undo_restores_tree_exactly`, `undo_keep_original_removes_results` |
| CLI `run` / `undo` | Task 7 |
