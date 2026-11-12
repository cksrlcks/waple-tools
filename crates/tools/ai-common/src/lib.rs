//! AI 도구 공용: DLL 안에 든 ONNX 모델의 세션.
//! DirectML(GPU)을 먼저 쓰고, 쓸 수 없거나 실패하면 CPU로 만든다. 세션은 모델마다 프로세스에 하나,
//! 추론은 한 번에 하나씩 (엔진이 여러 파일을 병렬로 돌려도 메모리가 파일 수만큼 늘지 않게).
//!
//! 도구 DLL은 DirectML.dll·d3d12.dll·dxgi.dll을 지연 로드한다(도구 build.rs). 지연 로드가 실패하면
//! 프로세스가 죽으므로, DirectML을 등록하기 전에 [`directml_usable`]로 DLL과 함수가 있는지 먼저 본다.

use std::ffi::{c_char, c_void};
use std::sync::{Mutex, OnceLock};

use ort::environment::Environment;
use ort::ep::DirectML;
use ort::session::{InMemorySession, Session as OrtSession};
use ort::value::Tensor;
use waple_tools_core::ToolError;

pub type Session = InMemorySession<'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Device {
    /// DirectML
    Gpu,
    Cpu,
}

type Shared = Result<(Mutex<Session>, Device), String>;

/// DLL 안에 든 모델. `bytes`가 비었으면 모델 없이 빌드된 것
pub struct Model {
    bytes: &'static [u8],
    input: [usize; 4],
    prefer: Device,
    shared: OnceLock<Shared>,
}

fn failed(e: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(format!("AI 추론에 실패했습니다: {e}"))
}

impl Model {
    /// `input`: 모델 입력 모양 (NCHW, 고정).
    /// `prefer`: `Gpu`면 DirectML을 먼저 시도하고 실패하면 CPU, `Cpu`면 처음부터 CPU만
    pub const fn new(bytes: &'static [u8], input: [usize; 4], prefer: Device) -> Self {
        Self {
            bytes,
            input,
            prefer,
            shared: OnceLock::new(),
        }
    }

    /// 장치를 정해 새 세션을 만든다 (공유 세션과 별개. 테스트에서 CPU 강제용)
    pub fn open(&self, device: Device) -> Result<Session, String> {
        if self.bytes.is_empty() {
            return Err("이 빌드에는 AI 모델이 들어 있지 않습니다.".into());
        }
        // onnxruntime 환경은 프로세스(= DLL에 든 onnxruntime 사본)당 하나만 만들 수 있다.
        // 원격 측정(Windows ETW)은 끈다
        static ENV: OnceLock<Result<Environment, String>> = OnceLock::new();
        let env = ENV
            .get_or_init(|| {
                ort::init()
                    .with_telemetry(false)
                    .build()
                    .map_err(|e| e.to_string())
            })
            .as_ref()
            .map_err(|e| format!("AI 모델을 불러올 수 없습니다: {e}"))?;
        let open = || -> ort::Result<Session> {
            let mut b = OrtSession::builder(env)?;
            if device == Device::Gpu {
                // DirectML EP는 병렬 실행·메모리 패턴을 지원하지 않는다 (onnxruntime 문서)
                b = b
                    .with_parallel_execution(false)?
                    .with_memory_pattern(false)?
                    .with_execution_providers([DirectML::default().build().error_on_failure()])?;
            }
            b.commit_from_memory_directly(self.bytes)
        };
        open().map_err(|e| format!("AI 모델을 불러올 수 없습니다: {e}"))
    }

    /// GPU 세션을 만들고 0으로 채운 입력으로 한 번 돌려 본다. 오래된 DirectML은 여기서 걸러진다
    fn open_gpu(&self) -> Option<Session> {
        if !directml_usable() {
            return None;
        }
        let mut s = self.open(Device::Gpu).ok()?;
        let zeros = vec![0f32; self.input.iter().product()];
        self.infer(&mut s, zeros).ok()?;
        Some(s)
    }

    fn create(&self) -> Shared {
        if self.prefer == Device::Gpu
            && let Some(s) = self.open_gpu()
        {
            return Ok((Mutex::new(s), Device::Gpu));
        }
        Ok((Mutex::new(self.open(Device::Cpu)?), Device::Cpu))
    }

    /// 공유 세션을 잠그고 `f`를 실행한다. 처음 부를 때 세션을 만든다.
    pub fn with_session<R>(
        &self,
        f: impl FnOnce(&mut Session) -> Result<R, ToolError>,
    ) -> Result<R, ToolError> {
        let (s, _) = self
            .shared
            .get_or_init(|| self.create())
            .as_ref()
            .map_err(|e| ToolError::Failed(e.clone()))?;
        let mut s = s.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut s)
    }

    /// 공유 세션이 쓰는 장치 (세션을 만든 뒤에만)
    pub fn device(&self) -> Option<Device> {
        self.shared.get()?.as_ref().ok().map(|(_, d)| *d)
    }

    /// 입력(NCHW, `input` 모양) → 첫 번째 출력 텐서
    pub fn infer(&self, s: &mut Session, input: Vec<f32>) -> Result<Vec<f32>, ToolError> {
        let tensor = Tensor::from_array((self.input, input)).map_err(failed)?;
        let outputs = s.run(ort::inputs![tensor]).map_err(failed)?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>().map_err(failed)?;
        Ok(data.to_vec())
    }
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}

/// 함수 이름 또는 서수(`#2`)
enum Proc {
    Name(&'static std::ffi::CStr),
    Ordinal(usize),
}

/// `dll`을 읽을 수 있고 `procs`가 모두 있는지
fn dll_has(dll: &str, procs: &[Proc]) -> bool {
    let wide: Vec<u16> = dll.encode_utf16().chain([0]).collect();
    // SAFETY: NUL로 끝나는 이름을 넘기고, 연 핸들은 여기서 닫는다
    unsafe {
        let h = LoadLibraryW(wide.as_ptr());
        if h.is_null() {
            return false;
        }
        let ok = procs.iter().all(|p| {
            let name = match p {
                Proc::Name(n) => n.as_ptr(),
                // MAKEINTRESOURCE: 서수는 하위 16비트 정수를 포인터로 넘긴다
                Proc::Ordinal(o) => *o as *const c_char,
            };
            !GetProcAddress(h, name).is_null()
        });
        FreeLibrary(h);
        ok
    }
}

/// onnxruntime의 DirectML EP가 지연 로드로 가져오는 DLL·함수가 모두 있는지 (`dumpbin /imports` 기준)
pub fn directml_usable() -> bool {
    dll_has(
        "DirectML.dll",
        &[Proc::Ordinal(2), Proc::Name(c"DMLCreateDevice1")],
    ) && dll_has(
        "d3d12.dll",
        &[
            Proc::Name(c"D3D12CreateDevice"),
            Proc::Name(c"D3D12SerializeVersionedRootSignature"),
        ],
    ) && dll_has("dxgi.dll", &[Proc::Name(c"CreateDXGIFactory2")])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dll_check_sees_functions_and_missing_dlls() {
        assert!(dll_has("kernel32.dll", &[Proc::Name(c"LoadLibraryW")]));
        assert!(!dll_has("kernel32.dll", &[Proc::Name(c"NoSuchFunction")]));
        assert!(!dll_has("waple-no-such.dll", &[]));
    }

    #[test]
    fn empty_model_is_missing_model() {
        let m = Model::new(&[], [1, 3, 8, 8], Device::Cpu);
        let r = m.with_session(|_| Ok(()));
        assert!(
            matches!(r, Err(ToolError::Failed(e)) if e == "이 빌드에는 AI 모델이 들어 있지 않습니다.")
        );
        assert_eq!(m.device(), None);
    }
}
