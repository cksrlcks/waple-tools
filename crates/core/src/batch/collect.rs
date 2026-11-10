//! 입력(파일·폴더 혼합)을 모아 처리 계획을 만든다. 파일 시스템은 바꾸지 않는다.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{JobSpec, OutputPolicy, Plan, PlannedFile, Skipped};
use crate::Tool;

/// 백업 폴더 이름
pub const BACKUP_DIR: &str = "original";
/// 실행 중 결과를 먼저 쓰는 임시 폴더 접두사
pub const TMP_PREFIX: &str = ".waple-tools-tmp-";

/// 상대 경로는 현재 폴더 기준 절대 경로로. 기록에 절대 경로가 남아야 다른 폴더에서 되돌려도 맞는다.
/// (`canonicalize`는 `\\?\` 접두사가 붙어 쓰지 않는다)
fn absolute(p: &Path) -> PathBuf {
    std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf())
}

pub fn plan(tool: &dyn Tool, spec: &JobSpec) -> Plan {
    let out_dir = match &spec.policy {
        OutputPolicy::OutputFolder { dir } => Some(absolute(dir)),
        _ => None,
    };
    let exts = tool.meta().extensions;
    let mut out = Plan::default();
    let mut seen = HashSet::new();
    let mut push = |out: &mut Plan, input: PathBuf, root: &Path| {
        let key = std::fs::canonicalize(&input).unwrap_or_else(|_| input.clone());
        if !seen.insert(key) {
            return;
        }
        if !supported(&input, &exts) {
            out.skipped.push(Skipped {
                path: input,
                reason: "지원하지 않는 형식".into(),
            });
            return;
        }
        let rel = input.strip_prefix(root).unwrap_or(&input).to_path_buf();
        let dest_dir = match &out_dir {
            Some(dir) => dir.join(rel.parent().unwrap_or(Path::new(""))),
            None => input.parent().unwrap_or(root).to_path_buf(),
        };
        out.files.push(PlannedFile {
            input,
            root: root.to_path_buf(),
            rel,
            dest_dir,
        });
    };

    for input in spec.inputs.iter().map(|p| absolute(p)) {
        if input.is_dir() {
            let mut files = Vec::new();
            walk(&input, spec.recursive, &mut files);
            for f in files {
                push(&mut out, f, &input);
            }
        } else if input.is_file() {
            let root = input.parent().unwrap_or(Path::new("")).to_path_buf();
            push(&mut out, input.clone(), &root);
        } else {
            out.skipped.push(Skipped {
                path: input,
                reason: "파일을 찾을 수 없음".into(),
            });
        }
    }
    out
}

/// 이름순으로 파일을 모은다. 제외 폴더(original·숨김·임시)는 들어가지 않는다.
fn walk(dir: &Path, recursive: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for p in paths {
        if p.is_dir() {
            if recursive && !excluded_dir(&p) {
                walk(&p, recursive, out);
            }
        } else {
            out.push(p);
        }
    }
}

fn excluded_dir(p: &Path) -> bool {
    let name = p.file_name().unwrap_or_default().to_string_lossy();
    name == BACKUP_DIR || name.starts_with('.') || name.starts_with(TMP_PREFIX) || hidden(p)
}

#[cfg(windows)]
fn hidden(p: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    std::fs::metadata(p).is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

#[cfg(not(windows))]
fn hidden(_: &Path) -> bool {
    false
}

fn supported(p: &Path, exts: &[String]) -> bool {
    exts.is_empty()
        || p.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .is_some_and(|e| exts.contains(&e))
}
