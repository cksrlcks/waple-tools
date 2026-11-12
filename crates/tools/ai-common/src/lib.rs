//! AI 도구 공용: DLL 안에 든 ONNX 모델의 세션.
//! `Device::Gpu`를 고른 모델은 DirectML을 먼저 쓰고, 쓸 수 없거나 실패하면 CPU로 바꾼다.
//! 세션은 모델마다 프로세스에 하나, 추론은 한 번에 하나씩 (엔진이 여러 파일을 병렬로 돌려도
//! 메모리가 파일 수만큼 늘지 않게).
//!
//! 도구 DLL은 DirectML.dll·d3d12.dll·dxgi.dll을 지연 로드한다(도구 build.rs). 지연 로드가 실패하면
//! 프로세스가 죽으므로, 쓰기 전에 DLL과 실제로 가져오는 함수가 있는지 먼저 본다.

use std::ffi::{c_char, c_void};
use std::sync::{Mutex, OnceLock};

use ort::environment::Environment;
use ort::ep::{CPU, DirectML};
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

/// DLL 안에 든 모델. `bytes`가 비었으면 모델 없이 빌드된 것
pub struct Model {
    bytes: &'static [u8],
    input: [usize; 4],
    prefer: Device,
    /// 공유 세션. 만들다 실패하면 비워 둔다(다음 호출에서 다시 만든다)
    state: Mutex<Option<(Session, Device)>>,
}

fn failed(e: impl std::fmt::Display) -> ToolError {
    ToolError::Failed(format!("AI 추론에 실패했습니다: {e}"))
}

/// 공유 세션을 잡은 동안 추론하는 손잡이. GPU 추론이 실패하면 CPU 세션으로 바꿔 한 번 더 한다
pub struct Runner<'a> {
    model: &'a Model,
    slot: &'a mut (Session, Device),
}

impl Runner<'_> {
    pub fn device(&self) -> Device {
        self.slot.1
    }

    pub fn infer(&mut self, input: Vec<f32>) -> Result<Vec<f32>, ToolError> {
        let retry = (self.slot.1 == Device::Gpu).then(|| input.clone());
        match (self.model.run(&mut self.slot.0, input), retry) {
            // GPU 드라이버 시간 초과(TDR)·장치 제거 등: 이후로는 CPU를 쓴다
            (Err(_), Some(input)) => {
                *self.slot = (self.model.open(Device::Cpu)?, Device::Cpu);
                self.model.run(&mut self.slot.0, input)
            }
            (r, _) => r,
        }
    }
}

impl Model {
    /// `input`: 모델 입력 모양 (NCHW, 고정).
    /// `prefer`: `Gpu`면 DirectML을 먼저 시도하고 실패하면 CPU, `Cpu`면 처음부터 CPU만
    pub const fn new(bytes: &'static [u8], input: [usize; 4], prefer: Device) -> Self {
        Self {
            bytes,
            input,
            prefer,
            state: Mutex::new(None),
        }
    }

    /// 장치를 정해 새 세션을 만든다 (공유 세션과 별개. 테스트에서 CPU 강제용)
    pub fn open(&self, device: Device) -> Result<Session, ToolError> {
        if self.bytes.is_empty() {
            return Err(ToolError::Failed(
                "이 빌드에는 AI 모델이 들어 있지 않습니다.".into(),
            ));
        }
        let env = environment().map_err(ToolError::Failed)?;
        let open = || -> ort::Result<Session> {
            // 정적 세션이 최대 사용량만큼 메모리를 계속 쥐지 않게 CPU arena·메모리 패턴을 끈다.
            // DirectML EP는 병렬 실행·메모리 패턴을 지원하지 않는다 (onnxruntime 문서)
            let cpu = CPU::default().with_arena_allocator(false).build();
            let mut b = OrtSession::builder(env)?
                .with_parallel_execution(false)?
                .with_memory_pattern(false)?;
            b = if device == Device::Gpu {
                b.with_execution_providers([DirectML::default().build().error_on_failure(), cpu])?
            } else {
                b.with_execution_providers([cpu])?
            };
            b.commit_from_memory_directly(self.bytes)
        };
        open().map_err(|e| ToolError::Failed(format!("AI 모델을 불러올 수 없습니다: {e}")))
    }

    /// GPU 세션을 만들고 0으로 채운 입력으로 한 번 돌려 본다. 오래된 DirectML은 여기서 걸러진다
    fn open_gpu(&self) -> Option<Session> {
        if !directml_usable() {
            return None;
        }
        let mut s = self.open(Device::Gpu).ok()?;
        let zeros = vec![0f32; self.input.iter().product()];
        self.run(&mut s, zeros).ok()?;
        Some(s)
    }

    fn create(&self) -> Result<(Session, Device), ToolError> {
        if self.prefer == Device::Gpu
            && let Some(s) = self.open_gpu()
        {
            return Ok((s, Device::Gpu));
        }
        Ok((self.open(Device::Cpu)?, Device::Cpu))
    }

    /// 공유 세션을 잠그고 `f`를 실행한다. 세션이 없으면 만든다.
    pub fn with_session<R>(
        &self,
        f: impl FnOnce(&mut Runner) -> Result<R, ToolError>,
    ) -> Result<R, ToolError> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.is_none() {
            *state = Some(self.create()?);
        }
        let slot = state.as_mut().expect("방금 채움");
        f(&mut Runner { model: self, slot })
    }

    /// 공유 세션이 쓰는 장치 (세션을 만든 뒤에만)
    pub fn device(&self) -> Option<Device> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.as_ref().map(|(_, d)| *d)
    }

    /// 세션 하나로 추론: 입력(NCHW, `input` 모양) → 첫 번째 출력 텐서
    pub fn run(&self, s: &mut Session, input: Vec<f32>) -> Result<Vec<f32>, ToolError> {
        let tensor = Tensor::from_array((self.input, input)).map_err(failed)?;
        let outputs = s.run(ort::inputs![tensor]).map_err(failed)?;
        let (_, data) = outputs[0].try_extract_tensor::<f32>().map_err(failed)?;
        Ok(data.to_vec())
    }
}

/// onnxruntime 환경: 프로세스(= DLL에 든 onnxruntime 사본)당 하나만 만들 수 있다
fn environment() -> Result<&'static Environment, String> {
    static ENV: OnceLock<Result<Environment, String>> = OnceLock::new();
    ENV.get_or_init(|| {
        // 환경 초기화가 지연 로드한 d3d12·dxgi를 부를 수 있다: 없으면 프로세스가 죽기 전에 멈춘다
        if !d3d12_usable() || !dxgi_usable() {
            return Err(
                "이 PC에는 AI 도구에 필요한 DirectX 12 구성 요소가 없습니다. Windows 10 이상에서 써 주세요."
                    .into(),
            );
        }
        // onnxruntime 스레드가 도는 동안 앱이 이 DLL을 내리면(업데이트·제거) 해제된 코드를 실행하게 된다
        pin_this_module();
        // 원격 측정(Windows ETW)은 끈다
        ort::init()
            .with_telemetry(false)
            .build()
            .map_err(|e| format!("AI 모델을 불러올 수 없습니다: {e}"))
    })
    .as_ref()
    .map_err(Clone::clone)
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
    fn GetModuleHandleExW(flags: u32, name: *const u16, module: *mut *mut c_void) -> i32;
}

/// 이 코드가 든 DLL을 프로세스가 끝날 때까지 내리지 않게 고정한다 (FreeLibrary가 무시된다)
fn pin_this_module() {
    const PIN: u32 = 0x1; // GET_MODULE_HANDLE_EX_FLAG_PIN
    const FROM_ADDRESS: u32 = 0x4; // GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
    let mut h = std::ptr::null_mut();
    // SAFETY: FROM_ADDRESS면 이름 대신 이 모듈 안의 주소(이 함수)를 넘긴다
    unsafe {
        GetModuleHandleExW(PIN | FROM_ADDRESS, pin_this_module as *const u16, &mut h);
    }
}

/// 함수 이름 또는 서수
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

// 아래 목록은 도구 DLL의 지연 로드 임포트와 같다 (`dumpbin /imports`, onnxruntime 1.31)

fn d3d12_usable() -> bool {
    dll_has(
        "d3d12.dll",
        &[
            Proc::Name(c"D3D12SerializeVersionedRootSignature"),
            Proc::Ordinal(101),
        ],
    )
}

fn dxgi_usable() -> bool {
    dll_has("dxgi.dll", &[Proc::Name(c"CreateDXGIFactory2")])
}

/// onnxruntime의 DirectML EP가 지연 로드로 가져오는 DLL·함수가 모두 있는지
pub fn directml_usable() -> bool {
    dll_has("DirectML.dll", &[Proc::Ordinal(2)]) && d3d12_usable() && dxgi_usable()
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
        // 실패는 남기지 않는다
        assert_eq!(m.device(), None);
    }
}
