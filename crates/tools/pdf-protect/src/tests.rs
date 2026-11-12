use serde_json::json;
use waple_tools_core::{JobContext, ToolOutput};
use waple_tools_pdf_common::lopdf::{Error, LoadOptions};
use waple_tools_pdf_common::testkit::{encrypted_pdf, page_labels, sample_pdf};

use super::*;

/// `bytes`를 `doc.pdf`로 두고 암호를 건 결과
fn run(bytes: &[u8], password: &str, confirm: &str) -> Result<Vec<u8>, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let result = tool().run(
        &ctx,
        std::slice::from_ref(&src),
        &out,
        &json!({ "password": password, "passwordConfirm": confirm }),
    );
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    let ToolOutput::Files(names) = result? else {
        panic!("파일이어야 함");
    };
    assert_eq!(names, [PathBuf::from("doc.pdf")]);
    Ok(std::fs::read(out.join(&names[0])).unwrap())
}

/// `password`로 열어 암호 없이 다시 저장한 바이트
fn open_with(bytes: &[u8], password: &str) -> Result<Vec<u8>, Error> {
    let mut doc = Document::load_mem_with_options(bytes, LoadOptions::with_password(password))?;
    let mut out = vec![];
    doc.save_to(&mut out).unwrap();
    Ok(out)
}

#[test]
fn locked_pdf_opens_only_with_password() {
    let src = sample_pdf(3, "");
    for password in ["secret-1", "한글 비밀번호"] {
        let out = run(&src, password, password).unwrap();
        for wrong in ["", "wrong"] {
            assert!(matches!(
                open_with(&out, wrong),
                Err(Error::InvalidPassword)
            ));
        }
        let plain = open_with(&out, password).unwrap();
        assert_eq!(page_labels(&plain), page_labels(&src), "{password}");
    }
}

#[test]
fn each_run_uses_a_new_key() {
    let src = sample_pdf(1, "");
    assert_ne!(run(&src, "a", "a").unwrap(), run(&src, "a", "a").unwrap());
}

#[test]
fn empty_or_mismatched_password_fails() {
    let src = sample_pdf(1, "");
    let e = run(&src, "", "").unwrap_err();
    assert!(e.to_string().contains("비밀번호를 입력하세요"), "{e}");
    let e = run(&src, "abc", "abd").unwrap_err();
    assert!(
        e.to_string().contains("비밀번호 확인이 맞지 않습니다"),
        "{e}"
    );
}

#[test]
fn already_encrypted_pdf_fails() {
    let e = run(&encrypted_pdf(1, "u", "o", true), "x", "x").unwrap_err();
    assert!(e.to_string().contains("이미 비밀번호가 걸린 PDF"), "{e}");
}

/// 넣을 때 확인 (`x-check-input`): 파일을 만들지 않고, 암호·손상 PDF는 오류
fn check(bytes: &[u8]) -> Result<ToolOutput, ToolError> {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("doc.pdf");
    std::fs::write(&src, bytes).unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let ctx = JobContext::new(|_| {}, Default::default());
    let r = tool().run(&ctx, &[src], &out, &json!({ "x-check-input": true }));
    assert_eq!(
        std::fs::read_dir(&out).unwrap().count(),
        0,
        "파일을 만들지 않음"
    );
    r
}

#[test]
fn check_input_rejects_encrypted_and_broken() {
    assert!(matches!(check(&sample_pdf(1, "")), Ok(ToolOutput::Blocks(b)) if b.is_empty()));
    for owner_only in [false, true] {
        let user = if owner_only { "" } else { "u" };
        let e = check(&encrypted_pdf(1, user, "o", true)).unwrap_err();
        assert!(e.to_string().contains("이미 비밀번호가 걸린 PDF"), "{e}");
    }
    let e = check(b"not a pdf").unwrap_err();
    assert!(e.to_string().contains("PDF를 읽을 수 없습니다"), "{e}");
}

#[test]
fn schema_has_two_password_fields() {
    let s = tool().options_schema();
    assert_eq!(s["properties"]["password"]["format"], "password");
    assert_eq!(s["properties"]["passwordConfirm"]["format"], "password");
    assert_eq!(s["x-allow-larger"], true);
    assert_eq!(s["x-files-only"], true);
    assert_eq!(s["x-keep-original"], true);
    assert_eq!(s["x-single-input"], true);
    assert_eq!(s["x-input-grid"], true);
    assert_eq!(s["x-check-input"], true);
}
