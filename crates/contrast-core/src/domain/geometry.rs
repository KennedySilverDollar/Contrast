//! 論理座標の幾何: `Point`、`Size`、`Rect`。
//!
//! 座標は i32、幅と高さは 1 以上。`Rect` は右端 `x + width` と下端 `y + height` が
//! i32 に収まることを構築時に保証する。全演算は checked / saturating 算術でパニックしない。
//! I5（Tiled 窓の矩形が作業領域内に収まり重ならない）を支える分割演算をここに置く。

use alloc::vec::Vec;

use crate::error::LayoutError;

/// 論理座標上の点。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    /// x 座標。
    pub x: i32,
    /// y 座標。
    pub y: i32,
}

impl Point {
    /// 座標 `(x, y)` の点を作る。
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 幅と高さ。どちらも 1 以上。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Size {
    /// 幅（1 以上）。
    width: i32,
    /// 高さ（1 以上）。
    height: i32,
}

impl Size {
    /// `width >= 1` かつ `height >= 1` のときだけ `Some` を返す。
    #[must_use]
    pub const fn new(width: i32, height: i32) -> Option<Self> {
        if width >= 1 && height >= 1 {
            Some(Self { width, height })
        } else {
            None
        }
    }

    /// 幅。
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.width
    }

    /// 高さ。
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.height
    }
}

/// 軸に平行な矩形。左上 `(x, y)` を含み、右端・下端を含まない半開区間。
///
/// 構築時に `x + width` と `y + height` が i32 に収まることを保証する。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// 左上の点。
    origin: Point,
    /// 大きさ。
    size: Size,
}

impl Rect {
    /// 幅・高さが 1 以上で、右端と下端が i32 に収まるときだけ `Some` を返す。
    #[must_use]
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Option<Self> {
        let Some(size) = Size::new(width, height) else {
            return None;
        };
        if x.checked_add(width).is_none() || y.checked_add(height).is_none() {
            return None;
        }
        Some(Self {
            origin: Point::new(x, y),
            size,
        })
    }

    /// 左端の x 座標。
    #[must_use]
    pub const fn x(&self) -> i32 {
        self.origin.x
    }

    /// 上端の y 座標。
    #[must_use]
    pub const fn y(&self) -> i32 {
        self.origin.y
    }

    /// 幅。
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.size.width
    }

    /// 高さ。
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.size.height
    }

    /// 右端 `x + width`（この列は含まない）。構築時の保証により飽和しない。
    #[must_use]
    pub const fn right(&self) -> i32 {
        self.origin.x.saturating_add(self.size.width)
    }

    /// 下端 `y + height`（この行は含まない）。構築時の保証により飽和しない。
    #[must_use]
    pub const fn bottom(&self) -> i32 {
        self.origin.y.saturating_add(self.size.height)
    }

    /// 中心 `(x + width / 2, y + height / 2)`。割り算は切り捨て。
    #[must_use]
    pub const fn center(&self) -> Point {
        Point::new(
            self.origin.x.saturating_add(half(self.size.width)),
            self.origin.y.saturating_add(half(self.size.height)),
        )
    }

    /// `p` が矩形内にあるか。右端・下端は含まない。
    #[must_use]
    pub const fn contains(&self, p: Point) -> bool {
        self.x() <= p.x && p.x < self.right() && self.y() <= p.y && p.y < self.bottom()
    }

    /// 2 つの矩形の共通部分。面積 0 なら `None`。
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Option<Self> {
        let left = self.x().max(other.x());
        let top = self.y().max(other.y());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        // 共通部分の幅・高さは元の幅・高さ以下なので i32 に収まる。
        let width = right.checked_sub(left)?;
        let height = bottom.checked_sub(top)?;
        Self::new(left, top, width, height)
    }

    /// 幅を `n` 列に分ける。商を各列に、余りを先頭から 1 ずつ配る。
    ///
    /// # Errors
    /// `n == 0` または `width < n` なら `LayoutError::AreaTooSmall`。
    pub fn split_columns(&self, n: usize) -> Result<Vec<Self>, LayoutError> {
        let spans = split_span(self.x(), self.width(), n)?;
        spans
            .into_iter()
            .map(|(x, w)| Self::new(x, self.y(), w, self.height()))
            .collect::<Option<Vec<_>>>()
            .ok_or(LayoutError::AreaTooSmall)
    }

    /// 高さを `n` 行に分ける。商を各行に、余りを先頭から 1 ずつ配る。
    ///
    /// # Errors
    /// `n == 0` または `height < n` なら `LayoutError::AreaTooSmall`。
    pub fn split_rows(&self, n: usize) -> Result<Vec<Self>, LayoutError> {
        let spans = split_span(self.y(), self.height(), n)?;
        spans
            .into_iter()
            .map(|(y, h)| Self::new(self.x(), y, self.width(), h))
            .collect::<Option<Vec<_>>>()
            .ok_or(LayoutError::AreaTooSmall)
    }
}

/// `v / 2` を切り捨てで求める（`v >= 1` で使う）。
const fn half(v: i32) -> i32 {
    match v.checked_div(2) {
        Some(h) => h,
        None => 0,
    }
}

/// 区間 `[start, start + len)` を `n` 個の `(開始, 長さ)` に分ける。
///
/// 商を各要素に、余りを先頭から 1 ずつ配る。`n == 0` または `len < n` なら
/// `AreaTooSmall`。`start + len` が i32 に収まる前提なので途中の和は飽和しない。
fn split_span(start: i32, len: i32, n: usize) -> Result<Vec<(i32, i32)>, LayoutError> {
    let count = i32::try_from(n).map_err(|_| LayoutError::AreaTooSmall)?;
    if count < 1 || len < count {
        return Err(LayoutError::AreaTooSmall);
    }
    let base = len.checked_div(count).ok_or(LayoutError::AreaTooSmall)?;
    let extra = len.checked_rem(count).ok_or(LayoutError::AreaTooSmall)?;
    let mut spans = Vec::with_capacity(n);
    let mut pos = start;
    for i in 0..count {
        let piece = if i < extra {
            base.saturating_add(1)
        } else {
            base
        };
        spans.push((pos, piece));
        pos = pos.saturating_add(piece);
    }
    Ok(spans)
}
