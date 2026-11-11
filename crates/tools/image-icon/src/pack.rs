//! 여러 크기의 PNG를 한 파일로 묶는다. ICO(Windows Vista+)와 ICNS(macOS 10.7+) 모두 PNG를 그대로 담을 수 있다.

/// `.ico`: 머리(6) + 항목(16 × 개수) + PNG들. 256px은 크기 칸에 0을 쓴다.
pub fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // 예약
    out.extend_from_slice(&1u16.to_le_bytes()); // 1 = 아이콘
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in images {
        let side = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[side, side, 0, 0]); // 가로, 세로, 팔레트 색 수, 예약
        out.extend_from_slice(&1u16.to_le_bytes()); // 색 평면
        out.extend_from_slice(&32u16.to_le_bytes()); // 비트 수
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// `.icns`에 넣는 (형식 코드, 픽셀 크기). 레티나(@2x) 칸도 같은 PNG를 쓴다.
pub const ICNS_TYPES: &[(&[u8; 4], u32)] = &[
    (b"icp4", 16),
    (b"icp5", 32),
    (b"ic11", 32), // 16@2x
    (b"icp6", 64),
    (b"ic12", 64), // 32@2x
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic13", 256), // 128@2x
    (b"ic09", 512),
    (b"ic14", 512),  // 256@2x
    (b"ic10", 1024), // 512@2x
];

/// `.icns`: `icns` + 전체 길이(BE) + (형식 코드 + 항목 길이(BE, 머리 8 포함) + PNG)…
pub fn icns(entries: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let total: usize = 8 + entries.iter().map(|(_, d)| 8 + d.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(total as u32).to_be_bytes());
    for (code, data) in entries {
        out.extend_from_slice(*code);
        out.extend_from_slice(&((8 + data.len()) as u32).to_be_bytes());
        out.extend_from_slice(data);
    }
    out
}
