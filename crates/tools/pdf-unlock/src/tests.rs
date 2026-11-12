use serde_json::json;
use waple_tools_core::{JobContext, ToolOutput};
use waple_tools_pdf_common::testkit::{encrypted_pdf, page_labels, sample_pdf};

use super::*;

/// `bytes`를 `doc.pdf`로 두고 `password`로 푼 결과
fn run(bytes: &[u8], password: &str) -> Result<Vec<u8>, ToolError> {
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
        &json!({ "password": password }),
    );
    assert_eq!(std::fs::read(&src).unwrap(), bytes, "원본 그대로");
    let ToolOutput::Files(names) = result? else {
        panic!("파일이어야 함");
    };
    assert_eq!(names, [PathBuf::from("doc.pdf")]);
    Ok(std::fs::read(out.join(&names[0])).unwrap())
}

/// 결과가 암호 없이 열리고 원래 글자가 그대로인가
fn assert_unlocked(out: &[u8]) {
    let doc = Document::load_mem(out).unwrap();
    assert!(!doc.was_encrypted() && !doc.is_encrypted());
    assert_eq!(page_labels(out), page_labels(&sample_pdf(2, "")));
}

#[test]
fn user_password_unlocks_aes256_and_rc4() {
    for aes in [true, false] {
        let src = encrypted_pdf(2, "열기1234", "owner", aes);
        assert_unlocked(&run(&src, "열기1234").unwrap());
    }
}

#[test]
fn owner_password_unlocks() {
    for aes in [true, false] {
        let src = encrypted_pdf(2, "user", "owner-pw", aes);
        assert_unlocked(&run(&src, "owner-pw").unwrap());
    }
}

#[test]
fn owner_only_pdf_needs_owner_password() {
    for aes in [true, false] {
        let src = encrypted_pdf(2, "", "owner", aes);
        // 한글만 쓴 비밀번호는 RC4에서 빈 값으로 바뀌어 빈 여는 암호와 맞을 뻔했다
        for wrong in ["wrong", "아무거나"] {
            let e = run(&src, wrong).unwrap_err();
            assert_eq!(
                e.to_string(),
                wrong_password().to_string(),
                "aes={aes} {wrong}"
            );
        }
        assert_unlocked(&run(&src, "owner").unwrap());
    }
}

#[test]
fn wrong_password_is_korean_error() {
    for aes in [true, false] {
        let src = encrypted_pdf(2, "user", "owner", aes);
        let e = run(&src, "틀림").unwrap_err();
        assert!(e.to_string().contains("비밀번호가 맞지 않습니다"), "{e}");
    }
}

#[test]
fn empty_password_asks_for_one() {
    let e = run(&encrypted_pdf(1, "user", "owner", true), "").unwrap_err();
    assert!(e.to_string().contains("비밀번호를 입력하세요"), "{e}");
}

#[test]
fn plain_pdf_and_garbage_fail() {
    let e = run(&sample_pdf(1, ""), "x").unwrap_err();
    assert!(e.to_string().contains("암호가 걸리지 않은"), "{e}");
    let e = run(b"not a pdf", "x").unwrap_err();
    assert!(e.to_string().contains("읽을 수 없습니다"), "{e}");
}

#[test]
fn schema_marks_password_field() {
    let schema = tool().options_schema();
    assert_eq!(schema["properties"]["password"]["format"], "password");
    assert_eq!(schema["x-allow-larger"], true);
    assert_eq!(schema["x-files-only"], true);
    assert_eq!(schema["x-keep-original"], true);
    assert_eq!(schema["x-input-grid"], true);
    assert_eq!(schema["x-single-input"], true);
}
