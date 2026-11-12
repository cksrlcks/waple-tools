//! model/det.onnx·rec.onnx가 있으면 `has_model`을 켜서 DLL에 넣는다. 없으면 도구는 "모델이 없다" 오류를 낸다.
//! 추가 설치용 DLL(addon)은 모델 없이 만들면 안 되므로 빌드를 멈춘다.
//! DirectML·D3D12·DXGI는 지연 로드한다(ort를 다른 AI 도구와 같은 기능으로 링크하므로). 이 도구는 CPU만 쓴다.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_model)");
    // 폴더를 본다: 없는 파일을 지정하면 빌드마다 다시 돈다
    println!("cargo::rerun-if-changed=model");
    let has = ["model/det.onnx", "model/rec.onnx"]
        .iter()
        .all(|p| std::path::Path::new(p).is_file());
    if has {
        println!("cargo::rustc-cfg=has_model");
    } else if std::env::var_os("CARGO_FEATURE_ADDON").is_some() {
        panic!(
            "model/det.onnx·rec.onnx가 없습니다. scripts/fetch-models.sh a11y.alt_text 로 받은 뒤 빌드하세요."
        );
    }
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        for dll in ["DirectML.dll", "d3d12.dll", "dxgi.dll"] {
            println!("cargo::rustc-link-arg=/DELAYLOAD:{dll}");
        }
        println!("cargo::rustc-link-arg=delayimp.lib");
    }
}
