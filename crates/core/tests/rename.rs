//! 파일명 일괄 변경 엔진 (batch::rename). 모두 임시 폴더에서 실제 파일로 확인한다.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use serde_json::json;
use waple_tools_core::batch::rename::{self, RenameStatus};
use waple_tools_core::batch::{self, FileResult, JobSpec, UndoItem};
use waple_tools_core::{JobContext, Tool, ToolError, ToolKind, ToolMeta, ToolOutput};

/// 옵션 `{"map": {"지금 이름": "새 이름"}}`대로 이름을 정하는 도구 대역 (없는 이름은 그대로)
struct MapTool;

impl Tool for MapTool {
    fn meta(&self) -> ToolMeta {
        ToolMeta {
            id: "test.rename".into(),
            name: "테스트 이름 변경".into(),
            category: "test".into(),
            description: String::new(),
            extensions: vec![],
        }
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Rename
    }
    fn options_schema(&self) -> serde_json::Value {
        json!({})
    }
    fn run(
        &self,
        _: &JobContext,
        input: &[PathBuf],
        _: &Path,
        opts: &serde_json::Value,
    ) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::Files(
            input
                .iter()
                .map(|p| {
                    let name = p.file_name().unwrap().to_string_lossy().to_string();
                    PathBuf::from(opts["map"][&name].as_str().unwrap_or(&name))
                })
                .collect(),
        ))
    }
}

fn spec(inputs: Vec<PathBuf>, map: serde_json::Value) -> JobSpec {
    serde_json::from_value(json!({ "inputs": inputs, "opts": { "map": map } })).unwrap()
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(&p, body).unwrap();
    p
}

/// 폴더 안 파일 이름 (정렬). 실제 대소문자 그대로
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

fn apply(spec: &JobSpec, logs: &Path, job: &str) -> Vec<FileResult> {
    rename::apply(
        &MapTool,
        spec,
        job,
        logs,
        Arc::new(AtomicBool::new(false)),
        |_| {},
    )
    .unwrap()
    .files
}

fn reason(s: &RenameStatus) -> &str {
    match s {
        RenameStatus::Conflict { reason } => reason,
        other => panic!("충돌이어야 함: {other:?}"),
    }
}

#[test]
fn plan_marks_changed_and_unchanged_without_touching_files() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "A");
    let c = write(d.path(), "c.txt", "C");
    let p = rename::plan(
        &MapTool,
        &spec(vec![a.clone(), c], json!({ "a.txt": "b.txt" })),
    )
    .unwrap();
    assert_eq!(p.rows[0].status, RenameStatus::Changed);
    assert_eq!(p.rows[0].to, d.path().join("b.txt"));
    assert_eq!(p.rows[1].status, RenameStatus::Unchanged);
    assert_eq!(names(d.path()), ["a.txt", "c.txt"]);
    let v = serde_json::to_value(&p.rows[0]).unwrap();
    assert_eq!(v["status"], "changed");
    assert_eq!(v["name"], "b.txt");
}

#[test]
fn plan_reports_each_conflict_kind() {
    let d = tempfile::tempdir().unwrap();
    let files: Vec<PathBuf> = ["1.txt", "2.txt", "3.txt", "4.txt", "5.txt", "6.txt"]
        .iter()
        .map(|n| write(d.path(), n, n))
        .collect();
    write(d.path(), "taken.txt", "남의 파일");
    let p = rename::plan(
        &MapTool,
        &spec(
            files,
            json!({
                "1.txt": "물음?.txt",
                "2.txt": "CON.txt",
                "3.txt": "끝점.",
                "4.txt": "same.txt",
                "5.txt": "same.txt",
                "6.txt": "taken.txt",
            }),
        ),
    )
    .unwrap();
    let r: Vec<&str> = p.rows.iter().map(|r| reason(&r.status)).collect();
    assert!(r[0].contains("쓸 수 없는 글자"), "{}", r[0]);
    assert!(r[1].contains("예약 이름"), "{}", r[1]);
    assert!(r[2].contains("끝에 점"), "{}", r[2]);
    assert_eq!(r[3], "다른 파일의 새 이름과 같음");
    assert_eq!(r[4], "다른 파일의 새 이름과 같음");
    assert_eq!(r[5], "같은 이름의 파일이 이미 있음");
}

#[cfg(windows)]
#[test]
fn duplicates_ignore_case_on_windows() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "");
    let b = write(d.path(), "b.txt", "");
    let p = rename::plan(
        &MapTool,
        &spec(vec![a, b], json!({ "a.txt": "x.txt", "b.txt": "X.TXT" })),
    )
    .unwrap();
    assert!(
        p.rows
            .iter()
            .all(|r| matches!(r.status, RenameStatus::Conflict { .. }))
    );
}

#[test]
fn a_name_freed_by_a_blocked_file_is_not_free() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "");
    let b = write(d.path(), "b.txt", "");
    // b는 쓸 수 없는 이름이라 그대로 남는다 → a가 b.txt로 갈 수 없다
    let p = rename::plan(
        &MapTool,
        &spec(vec![a, b], json!({ "a.txt": "b.txt", "b.txt": "b?.txt" })),
    )
    .unwrap();
    assert_eq!(reason(&p.rows[0].status), "같은 이름의 파일이 이미 있음");
}

#[test]
fn swap_then_undo() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "A");
    let b = write(d.path(), "b.txt", "B");
    let s = spec(
        vec![a.clone(), b.clone()],
        json!({ "a.txt": "b.txt", "b.txt": "a.txt" }),
    );
    assert!(
        rename::plan(&MapTool, &s)
            .unwrap()
            .rows
            .iter()
            .all(|r| r.status == RenameStatus::Changed)
    );
    let files = apply(&s, logs.path(), "swap");
    assert!(
        files.iter().all(|f| matches!(f, FileResult::Done { .. })),
        "{files:?}"
    );
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "B");
    assert_eq!(std::fs::read_to_string(&b).unwrap(), "A");
    assert_eq!(
        names(d.path()),
        ["a.txt", "b.txt"],
        "임시 이름이 남지 않는다"
    );

    let undo = batch::undo(logs.path(), "swap").unwrap();
    assert!(
        undo.iter().all(|i| matches!(i, UndoItem::Restored { .. })),
        "{undo:?}"
    );
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "A");
    assert_eq!(std::fs::read_to_string(&b).unwrap(), "B");
    assert_eq!(names(d.path()), ["a.txt", "b.txt"]);
}

#[test]
fn case_only_rename_then_undo() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let a = write(d.path(), "photo.jpg", "P");
    let s = spec(vec![a], json!({ "photo.jpg": "Photo.JPG" }));
    assert_eq!(
        rename::plan(&MapTool, &s).unwrap().rows[0].status,
        RenameStatus::Changed
    );
    let files = apply(&s, logs.path(), "case");
    assert!(matches!(files[0], FileResult::Done { .. }), "{files:?}");
    assert_eq!(names(d.path()), ["Photo.JPG"]);
    batch::undo(logs.path(), "case").unwrap();
    assert_eq!(names(d.path()), ["photo.jpg"]);
}

#[test]
fn chain_and_plain_renames_undo_in_order() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let inputs = vec![
        write(d.path(), "1.txt", "one"),
        write(d.path(), "2.txt", "two"),
        write(d.path(), "x.txt", "x"),
    ];
    // 1 → 2, 2 → 3 (빈 이름), x → y
    let s = spec(
        inputs,
        json!({ "1.txt": "2.txt", "2.txt": "3.txt", "x.txt": "y.txt" }),
    );
    apply(&s, logs.path(), "chain");
    assert_eq!(names(d.path()), ["2.txt", "3.txt", "y.txt"]);
    assert_eq!(
        std::fs::read_to_string(d.path().join("2.txt")).unwrap(),
        "one"
    );
    assert_eq!(
        std::fs::read_to_string(d.path().join("3.txt")).unwrap(),
        "two"
    );
    batch::undo(logs.path(), "chain").unwrap();
    assert_eq!(names(d.path()), ["1.txt", "2.txt", "x.txt"]);
    assert_eq!(
        std::fs::read_to_string(d.path().join("1.txt")).unwrap(),
        "one"
    );
    assert_eq!(
        std::fs::read_to_string(d.path().join("2.txt")).unwrap(),
        "two"
    );
}

#[test]
fn never_overwrites_an_unrelated_file() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "A");
    let taken = write(d.path(), "taken.txt", "남의 파일");
    let files = apply(
        &spec(vec![a.clone()], json!({ "a.txt": "taken.txt" })),
        logs.path(),
        "x",
    );
    assert!(matches!(files[0], FileResult::Failed { .. }));
    assert_eq!(std::fs::read_to_string(&taken).unwrap(), "남의 파일");
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "A");
}

#[test]
fn file_appearing_after_plan_fails_only_that_row() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "A");
    let b = write(d.path(), "b.txt", "B");
    let s = spec(
        vec![a.clone(), b],
        json!({ "a.txt": "new-a.txt", "b.txt": "new-b.txt" }),
    );
    let p = rename::plan(&MapTool, &s).unwrap();
    assert!(p.rows.iter().all(|r| r.status == RenameStatus::Changed));
    // 미리보기와 실행 사이에 다른 프로그램이 같은 이름을 만들었다
    let intruder = write(d.path(), "new-a.txt", "끼어든 파일");
    let files = apply(&s, logs.path(), "toctou");
    assert!(matches!(files[0], FileResult::Failed { .. }), "{files:?}");
    assert!(matches!(files[1], FileResult::Done { .. }), "{files:?}");
    assert_eq!(std::fs::read_to_string(&intruder).unwrap(), "끼어든 파일");
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "A");
    assert_eq!(names(d.path()), ["a.txt", "new-a.txt", "new-b.txt"]);
}

#[test]
fn folder_inputs_expand_and_unchanged_rows_are_skipped() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    write(d.path(), "a.txt", "");
    write(d.path(), "sub/b.txt", "");
    let mut s = spec(vec![d.path().to_path_buf()], json!({ "b.txt": "c.txt" }));
    assert_eq!(
        rename::plan(&MapTool, &s).unwrap().rows.len(),
        1,
        "하위 폴더 제외"
    );
    s.recursive = true;
    let p = rename::plan(&MapTool, &s).unwrap();
    assert_eq!(p.rows.len(), 2);
    let files = apply(&s, logs.path(), "dir");
    assert!(matches!(files[0], FileResult::Skipped { .. }));
    assert!(matches!(files[1], FileResult::Done { .. }));
    assert!(d.path().join("sub/c.txt").is_file());
}

#[test]
fn normal_run_rejects_rename_tools() {
    let d = tempfile::tempdir().unwrap();
    let logs = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.txt", "A");
    let report = batch::run(
        &MapTool,
        &spec(vec![a.clone()], json!({ "a.txt": "b.txt" })),
        "run",
        logs.path(),
        Default::default(),
        |_| {},
    )
    .unwrap();
    assert!(
        matches!(&report.files[0], FileResult::Failed { message, .. } if message == "지원하지 않는 도구 종류입니다.")
    );
    assert_eq!(names(d.path()), ["a.txt"]);
}
