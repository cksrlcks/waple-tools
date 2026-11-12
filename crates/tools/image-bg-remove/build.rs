//! model/model.onnx가 있으면 `has_model`을 켜서 DLL에 넣는다. 없으면 도구는 "모델이 없다" 오류를 낸다.
//! 추가 설치용 DLL(addon)은 모델 없이 만들면 안 되므로 빌드를 멈춘다.
//! DirectML·D3D12·DXGI는 지연 로드한다: 없는 PC에서도 DLL이 로드되고, 쓰기 전에 ai-common이 있는지 확인한다.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_model)");
    // 폴더(.gitkeep으로 늘 있음)를 본다: 없는 파일을 지정하면 빌드마다 다시 돈다
    println!("cargo::rerun-if-changed=model");
    if std::path::Path::new("model/model.onnx").is_file() {
        println!("cargo::rustc-cfg=has_model");
    } else if std::env::var_os("CARGO_FEATURE_ADDON").is_some() {
        panic!(
            "model/model.onnx가 없습니다. scripts/fetch-models.sh image.bg_remove 로 받은 뒤 빌드하세요."
        );
    }
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        // cdylib(addon)과 테스트 실행 파일 모두에 적용된다
        for dll in ["DirectML.dll", "d3d12.dll", "dxgi.dll"] {
            println!("cargo::rustc-link-arg=/DELAYLOAD:{dll}");
        }
        println!("cargo::rustc-link-arg=delayimp.lib");
    }
}
