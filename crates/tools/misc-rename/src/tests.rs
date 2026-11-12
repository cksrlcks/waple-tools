use std::path::PathBuf;

use serde_json::json;
use waple_tools_core::{JobContext, ToolError, ToolOutput};

use super::*;

/// 임시 폴더에 `files`를 만들고 그 순서대로 새 이름을 받는다. 파일은 그대로여야 한다
fn names(files: &[&str], opts: serde_json::Value) -> Result<Vec<String>, ToolError> {
    let d = tempfile::tempdir().unwrap();
    let inputs: Vec<PathBuf> = files
        .iter()
        .map(|n| {
            let p = d.path().join(n);
            std::fs::write(&p, n).unwrap();
            p
        })
        .collect();
    let ctx = JobContext::new(|_| {}, Default::default());
    let out = tool().run(&ctx, &inputs, d.path(), &opts)?;
    for (p, n) in inputs.iter().zip(files) {
        assert_eq!(
            std::fs::read_to_string(p).unwrap(),
            *n,
            "파일을 건드리지 않는다"
        );
    }
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), files.len());
    let ToolOutput::Files(names) = out else {
        panic!("이름 목록이어야 함");
    };
    Ok(names
        .into_iter()
        .map(|n| n.to_string_lossy().into_owned())
        .collect())
}

#[test]
fn nothing_on_keeps_names() {
    assert_eq!(names(&["a.txt", "b"], json!({})).unwrap(), ["a.txt", "b"]);
}

#[test]
fn find_replace_plain_and_case() {
    let o = json!({ "replace": true, "find": "IMG", "replaceWith": "photo" });
    assert_eq!(
        names(&["IMG_1.jpg", "img_2.jpg"], o).unwrap(),
        ["photo_1.jpg", "img_2.jpg"]
    );
    let o = json!({ "replace": true, "find": "img", "replaceWith": "$1", "matchCase": false });
    assert_eq!(
        names(&["IMG_1.jpg"], o).unwrap(),
        ["$1_1.jpg"],
        "정규식이 아니면 $1도 글자 그대로"
    );
    // 확장자는 바꾸지 않는다
    let o = json!({ "replace": true, "find": "jpg", "replaceWith": "x" });
    assert_eq!(names(&["jpg.jpg"], o).unwrap(), ["x.jpg"]);
}

#[test]
fn find_replace_regex_groups() {
    let o = json!({
        "replace": true, "regex": true,
        "find": r"(\d{4})(\d{2})(\d{2})", "replaceWith": "$1-$2-$3"
    });
    assert_eq!(
        names(&["scan20261112.pdf"], o).unwrap(),
        ["scan2026-11-12.pdf"]
    );
    let bad = json!({ "replace": true, "regex": true, "find": "(" });
    assert!(matches!(
        names(&["a.txt"], bad),
        Err(ToolError::InvalidOptions(_))
    ));
}

#[test]
fn case_and_spaces() {
    let lower = json!({ "changeCase": true, "caseMode": "lower" });
    assert_eq!(names(&["My File.TXT"], lower).unwrap(), ["my file.TXT"]);
    let title = json!({ "changeCase": true, "caseMode": "title", "spacesToHyphen": true });
    assert_eq!(
        names(&["  hello   wORLD_x.md"], title).unwrap(),
        ["Hello-World_X.md"]
    );
    let ext = json!({ "extCase": "lower" });
    assert_eq!(names(&["A.JPG"], ext).unwrap(), ["A.jpg"]);
}

#[test]
fn prefix_suffix_and_numbers_in_input_order() {
    let o = json!({
        "affix": true, "prefix": "x-", "suffix": "-y",
        "number": true, "start": 9, "digits": 3, "numberPosition": "before", "numberSeparator": "_"
    });
    assert_eq!(
        names(&["b.png", "a.png"], o).unwrap(),
        ["009_x-b-y.png", "010_x-a-y.png"]
    );
}

#[test]
fn empty_name_is_left_for_the_engine() {
    let o = json!({ "replace": true, "regex": true, "find": ".*" });
    assert_eq!(names(&["a.txt"], o).unwrap(), [""]);
    // 이름이 비어도 번호를 붙이면 번호만 (구분 글자 없이)
    let o = json!({ "replace": true, "regex": true, "find": ".*", "number": true });
    assert_eq!(names(&["a.txt"], o).unwrap(), ["001.txt"]);
}

#[test]
fn date_today_and_modified() {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let o = json!({ "date": true, "datePosition": "after", "dateSeparator": "_" });
    assert_eq!(names(&["a.txt"], o).unwrap(), [format!("a_{today}.txt")]);

    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("m.txt");
    std::fs::write(&p, "").unwrap();
    // 2020-06-15 12:00 UTC: 어느 시간대에서도 같은 날
    let t = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_592_222_400);
    std::fs::File::options()
        .write(true)
        .open(&p)
        .unwrap()
        .set_modified(t)
        .unwrap();
    let o: RenameOptions = serde_json::from_value(
        json!({ "date": true, "dateSource": "modified", "dateFormat": "compact" }),
    )
    .unwrap();
    assert_eq!(
        new_name(&p, 0, &o, None, Local::now()).unwrap(),
        "20200615_m.txt"
    );
}

#[test]
fn schema_groups_rules() {
    let s = tool().options_schema();
    for k in ["replace", "changeCase", "affix", "date", "number"] {
        assert_eq!(s["properties"][k]["x-group-toggle"], true, "{k}");
    }
    assert_eq!(
        s["properties"]["find"]["x-enabled-when"],
        json!({ "replace": [true] })
    );
    assert_eq!(s["properties"]["digits"]["maximum"], 8);
}
