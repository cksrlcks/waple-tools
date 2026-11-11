//! WOFF 1.0 인코딩: sfnt 테이블마다 zlib로 압축한다 (https://www.w3.org/TR/WOFF/)

use std::io::Write;

use flate2::{Compression, write::ZlibEncoder};

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn pad4(v: &mut Vec<u8>) {
    v.resize(v.len().next_multiple_of(4), 0);
}

/// sfnt(TTF·OTF) → WOFF. 테이블 디렉터리가 깨졌으면 None
pub fn encode(sfnt: &[u8]) -> Option<Vec<u8>> {
    let n = usize::from(be16(sfnt, 4)?);
    // (tag, checksum, data)
    let mut tables = (0..n)
        .map(|i| {
            let rec = 12 + 16 * i;
            let off = be32(sfnt, rec + 8)? as usize;
            let len = be32(sfnt, rec + 12)? as usize;
            Some((
                sfnt.get(rec..rec + 4)?,
                be32(sfnt, rec + 4)?,
                sfnt.get(off..off.checked_add(len)?)?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    // WOFF 디렉터리는 태그 오름차순이어야 한다
    tables.sort_by_key(|t| t.0);

    let mut dir = Vec::with_capacity(20 * n);
    let mut data = Vec::new();
    let data_start = 44 + 20 * n;
    let mut sfnt_size = 12 + 16 * n;
    for (tag, checksum, raw) in &tables {
        let mut z = ZlibEncoder::new(Vec::new(), Compression::best());
        z.write_all(raw).ok()?;
        let z = z.finish().ok()?;
        // 압축해도 줄지 않으면 그대로 넣는다 (표준 규칙)
        let stored: &[u8] = if z.len() < raw.len() { &z } else { raw };
        dir.extend_from_slice(tag);
        dir.extend_from_slice(&u32::try_from(data_start + data.len()).ok()?.to_be_bytes());
        dir.extend_from_slice(&u32::try_from(stored.len()).ok()?.to_be_bytes());
        dir.extend_from_slice(&u32::try_from(raw.len()).ok()?.to_be_bytes());
        dir.extend_from_slice(&checksum.to_be_bytes());
        data.extend_from_slice(stored);
        pad4(&mut data);
        sfnt_size += raw.len().next_multiple_of(4);
    }
    let total = data_start + data.len();

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"wOFF");
    out.extend_from_slice(sfnt.get(..4)?); // flavor
    out.extend_from_slice(&u32::try_from(total).ok()?.to_be_bytes());
    out.extend_from_slice(&u16::try_from(n).ok()?.to_be_bytes());
    out.extend_from_slice(&[0, 0]); // reserved
    out.extend_from_slice(&u32::try_from(sfnt_size).ok()?.to_be_bytes());
    out.extend_from_slice(&[0, 1, 0, 0]); // 버전 1.0
    out.extend_from_slice(&[0; 20]); // 메타데이터·비공개 데이터 없음
    out.extend_from_slice(&dir);
    out.extend_from_slice(&data);
    Some(out)
}
