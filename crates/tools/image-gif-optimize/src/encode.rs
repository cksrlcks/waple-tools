//! 프레임 간 최적화 저장: 앞 프레임과 같은 화소는 투명(앞 화면 유지)으로, 바뀐 영역만 잘라 쓴다 (gifsicle -O 방식)

use std::borrow::Cow;

use gif::{DisposalMethod, Repeat};
use waple_tools_core::{JobContext, ToolError};

use crate::Frame;
use crate::palette::Palette;

fn failed(_: gif::EncodingError) -> ToolError {
    ToolError::Failed("GIF를 저장하지 못했습니다.".into())
}

/// (x, y, 가로, 세로)
type Rect = (usize, usize, usize, usize);

/// `hit(i)`인 화소들을 감싸는 사각형
fn bbox(w: usize, len: usize, hit: impl Fn(usize) -> bool) -> Option<Rect> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for i in (0..len).filter(|&i| hit(i)) {
        let (x, y) = (i % w, i / w);
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    (x0 != usize::MAX).then(|| (x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

fn union((ax, ay, aw, ah): Rect, (bx, by, bw, bh): Rect) -> Rect {
    let (x, y) = (ax.min(bx), ay.min(by));
    (x, y, (ax + aw).max(bx + bw) - x, (ay + ah).max(by + bh) - y)
}

/// 잘라 쓴 프레임을 `to`(프레임 사각형을 포함) 크기로 편다. 늘어난 곳은 투명 = 앞 화면 유지라 보이는 결과는 같다
fn expand(f: &mut gif::Frame<'static>, to: Rect, t: u8) {
    let (x, y, w, h) = to;
    let mut buf = vec![t; w * h];
    let (fw, left, top) = (
        usize::from(f.width),
        usize::from(f.left),
        usize::from(f.top),
    );
    for (r, row) in f.buffer.chunks(fw).enumerate() {
        let at = (top - y + r) * w + left - x;
        buf[at..at + fw].copy_from_slice(row);
    }
    f.buffer = Cow::Owned(buf);
    (f.left, f.top, f.width, f.height) = (x as u16, y as u16, w as u16, h as u16);
}

pub(crate) fn write(
    frames: &[Frame],
    w: u32,
    h: u32,
    pal: &Palette,
    repeat: Repeat,
    ctx: &JobContext,
) -> Result<Vec<u8>, ToolError> {
    let (w, h) = (w as usize, h as usize);
    let t = pal.transparent;
    let mut enc = gif::Encoder::new(Vec::new(), w as u16, h as u16, &pal.rgb).map_err(failed)?;
    enc.set_repeat(repeat).map_err(failed)?;
    let mut canvas = vec![t; w * h];
    // 다음 프레임을 보고 나서 쓴다: 지우기(dispose)나 지연 시간을 고쳐야 할 수 있다
    let mut pending: Option<gif::Frame<'static>> = None;
    for (k, f) in frames.iter().enumerate() {
        ctx.check_cancel()?;
        let cur: Vec<u8> = f.rgba.pixels().map(|p| pal.index(p)).collect();
        let delay = u16::try_from(f.delay_ms.div_ceil(10)).unwrap_or(u16::MAX);
        // 투명으로 바뀌는 화소는 "앞 화면 유지"로 그릴 수 없다: 앞 프레임을 그 화소들까지 펴고, 보여 준 뒤 그 영역을 지우게 한다
        let to_clear = bbox(w, cur.len(), |i| cur[i] == t && canvas[i] != t);
        let clear = to_clear.is_some();
        if let (Some(c), Some(p)) = (to_clear, pending.as_mut()) {
            let own = (
                usize::from(p.left),
                usize::from(p.top),
                usize::from(p.width),
                usize::from(p.height),
            );
            let (x, y, rw, rh) = union(own, c);
            expand(p, (x, y, rw, rh), t);
            p.dispose = DisposalMethod::Background;
            for row in y..y + rh {
                canvas[row * w + x..row * w + x + rw].fill(t);
            }
        }
        let rect = bbox(w, cur.len(), |i| cur[i] != canvas[i]);
        match (rect, pending.as_mut()) {
            // 바뀐 게 없으면 앞 프레임을 그만큼 더 보여 준다
            (None, Some(p)) if !clear => p.delay = p.delay.saturating_add(delay),
            _ => {
                if let Some(p) = pending.take() {
                    enc.write_frame(&p).map_err(failed)?;
                }
                let (x, y, rw, rh) = rect.unwrap_or((0, 0, 1, 1));
                let mut buf = Vec::with_capacity(rw * rh);
                for row in y..y + rh {
                    let s = row * w + x..row * w + x + rw;
                    buf.extend(
                        cur[s.clone()]
                            .iter()
                            .zip(&canvas[s])
                            .map(|(&c, &old)| if c == old { t } else { c }),
                    );
                }
                pending = Some(gif::Frame {
                    delay,
                    dispose: DisposalMethod::Keep,
                    transparent: Some(t),
                    left: x as u16,
                    top: y as u16,
                    width: rw as u16,
                    height: rh as u16,
                    buffer: Cow::Owned(buf),
                    ..gif::Frame::default()
                });
            }
        }
        canvas = cur;
        ctx.progress((k + 1) as f32 / frames.len() as f32);
    }
    if let Some(p) = pending {
        enc.write_frame(&p).map_err(failed)?;
    }
    enc.into_inner().map_err(failed)
}
