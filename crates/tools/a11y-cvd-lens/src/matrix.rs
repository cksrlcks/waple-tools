//! 색각이상 시뮬레이션 행렬: Machado, Oliveira, Fernandes (2009), 심각도 1.0

/// 보기 방식 하나. `m`은 out = M · in (행 = 출력 채널 R·G·B)
pub struct Mode {
    pub name: &'static str,
    pub m: [[f32; 3]; 3],
}

/// 순서 고정: 옵션 enum·렌즈 메뉴·`1`~`4` 키가 이 순서를 쓴다
// ponytail: 감마 공간에 바로 적용하는 근사. Magnification API 색 행렬에는 선형화 단계를 넣을 수 없다
pub const MODES: [Mode; 4] = [
    Mode {
        name: "적색맹",
        m: [
            [0.152286, 1.052583, -0.204868],
            [0.114503, 0.786281, 0.099216],
            [-0.003882, -0.048116, 1.051998],
        ],
    },
    Mode {
        name: "녹색맹",
        m: [
            [0.367322, 0.860646, -0.227968],
            [0.280085, 0.672501, 0.047413],
            [-0.011820, 0.042940, 0.968881],
        ],
    },
    Mode {
        name: "청색맹",
        m: [
            [1.255528, -0.076749, -0.178779],
            [-0.078411, 0.930809, 0.147602],
            [0.004733, 0.691367, 0.303900],
        ],
    },
    Mode {
        name: "전색맹",
        m: [
            [0.299, 0.587, 0.114],
            [0.299, 0.587, 0.114],
            [0.299, 0.587, 0.114],
        ],
    },
];

/// 녹색맹 (가장 흔함)
pub const DEFAULT_MODE: usize = 1;

/// `MAGCOLOREFFECT.transform`: 5×5, 행 = 입력 채널, 열 = 출력 채널이라 전치한다. 알파·오프셋은 그대로
pub fn effect(m: &[[f32; 3]; 3]) -> [f32; 25] {
    let mut t = [0f32; 25];
    for (i, row) in t.chunks_exact_mut(5).take(3).enumerate() {
        for (j, cell) in row.iter_mut().take(3).enumerate() {
            *cell = m[j][i];
        }
    }
    t[18] = 1.0;
    t[24] = 1.0;
    t
}
