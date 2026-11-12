//! Real-ESRGAN x4plus 세션. 프로세스에 하나만 만들고, 추론은 한 번에 하나씩 한다
//! (엔진이 여러 파일을 병렬로 돌려도 메모리가 파일 수만큼 늘지 않게).

use std::sync::{Mutex, OnceLock};

use ort::session::InMemorySession;
use ort::value::Tensor;
use waple_tools_core::ToolError;

use crate::tile::TILE;

#[cfg(has_model)]
static MODEL: &[u8] = include_bytes!("../model/model.onnx");
#[cfg(not(has_model))]
static MODEL: &[u8] = &[];

pub type Session = InMemorySession<'static>;

fn create() -> Result<Mutex<Session>, String> {
    if MODEL.is_empty() {
        return Err("이 빌드에는 AI 모델이 들어 있지 않습니다.".into());
    }
    // 원격 측정(Windows ETW)은 끈다. 세션이 환경을 붙잡고 있어 지역 변수로 충분하다
    let session = ort::init()
        .with_telemetry(false)
        .build()
        .and_then(|env| ort::session::Session::builder(&env))
        .and_then(|mut b| b.commit_from_memory_directly(MODEL))
        .map_err(|e| format!("AI 모델을 불러올 수 없습니다: {e}"))?;
    Ok(Mutex::new(session))
}

/// 세션을 잠그고 `f`를 실행한다. 처음 부를 때 세션을 만든다.
pub fn with_session<R>(
    f: impl FnOnce(&mut Session) -> Result<R, ToolError>,
) -> Result<R, ToolError> {
    static SESSION: OnceLock<Result<Mutex<Session>, String>> = OnceLock::new();
    let shared = SESSION
        .get_or_init(create)
        .as_ref()
        .map_err(|e| ToolError::Failed(e.clone()))?;
    let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut s)
}

fn failed(e: ort::Error) -> ToolError {
    ToolError::Failed(format!("AI 추론에 실패했습니다: {e}"))
}

/// 타일 하나: NCHW `[1, 3, 128, 128]` → `[1, 3, 512, 512]` (0~1 RGB)
pub fn infer(s: &mut Session, input: &[f32]) -> Result<Vec<f32>, ToolError> {
    let t = TILE as usize;
    let tensor = Tensor::from_array(([1usize, 3, t, t], input.to_vec())).map_err(failed)?;
    let outputs = s.run(ort::inputs![tensor]).map_err(failed)?;
    let (_, data) = outputs[0].try_extract_tensor::<f32>().map_err(failed)?;
    Ok(data.to_vec())
}
