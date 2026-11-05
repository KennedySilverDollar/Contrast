//! 窓の矩形の計算（純粋な関数）。隙間（gap・outerGap）を core の矩形に重ねる。
//!
//! core が返すタイルは変えない。描画側が、タイルから「フレーム」（枠線の外側の矩形）を作り、窓はそこから枠の幅だけ内側に置く。

use contrast_core::Rect;

/// 四辺から `l`・`t`・`r`・`b` だけ縮めた矩形。縮め切れなければ元の矩形。
fn shrink(rect: Rect, l: i32, t: i32, r: i32, b: i32) -> Rect {
    Rect::new(
        rect.x().saturating_add(l),
        rect.y().saturating_add(t),
        rect.width().saturating_sub(l.saturating_add(r)),
        rect.height().saturating_sub(t.saturating_add(b)),
    )
    .unwrap_or(rect)
}

/// `rects` をすべて含む最小の矩形。空なら `None`。
pub fn bounds_of(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    let mut it = rects.into_iter();
    let first = it.next()?;
    let (mut l, mut t, mut r, mut b) = (first.x(), first.y(), first.right(), first.bottom());
    for rect in it {
        l = l.min(rect.x());
        t = t.min(rect.y());
        r = r.max(rect.right());
        b = b.max(rect.bottom());
    }
    Rect::new(l, t, r.saturating_sub(l), b.saturating_sub(t))
}

/// タイルから、隙間を引いたフレームを作る。`bounds` は同じ画面のタイル全体の外枠。
///
/// `bounds` の端に接する辺は `outer`、それ以外（隣の窓との境）は `gap` の半分を引く。
/// 隣り合う 2 枚の合計がちょうど `gap` になる（奇数のときは右/下の辺が 1 小さい）。
/// どちらも 0 ならタイルそのもの。引き切れないほど大きいときもタイルそのもの。
pub fn frame_rect(tile: Rect, bounds: Rect, gap: i32, outer: i32) -> Rect {
    let (gap, outer) = (gap.max(0), outer.max(0));
    if gap == 0 && outer == 0 {
        return tile;
    }
    let near = gap - gap / 2;
    let far = gap / 2;
    let left = if tile.x() <= bounds.x() { outer } else { near };
    let top = if tile.y() <= bounds.y() { outer } else { near };
    let right = if tile.right() >= bounds.right() {
        outer
    } else {
        far
    };
    let bottom = if tile.bottom() >= bounds.bottom() {
        outer
    } else {
        far
    };
    shrink(tile, left, top, right, bottom)
}

/// フレームから枠の幅 `border` だけ内側の、窓の矩形。
pub fn window_rect(frame: Rect, border: i32) -> Rect {
    let b = border.max(0);
    shrink(frame, b, b, b, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::new(x, y, w, h).expect("positive rect")
    }

    #[test]
    fn zero_gaps_leave_the_tile_alone() {
        let tile = r(10, 20, 300, 200);
        assert_eq!(frame_rect(tile, r(0, 0, 1000, 1000), 0, 0), tile);
    }

    #[test]
    fn single_window_gets_the_outer_gap_on_all_sides() {
        let area = r(0, 0, 1000, 600);
        assert_eq!(frame_rect(area, area, 16, 8), r(8, 8, 984, 584));
    }

    #[test]
    fn neighbours_are_separated_by_exactly_gap() {
        let area = r(0, 0, 1000, 600);
        let (a, b) = (r(0, 0, 500, 600), r(500, 0, 500, 600));
        let (fa, fb) = (frame_rect(a, area, 15, 4), frame_rect(b, area, 15, 4));
        assert_eq!(fb.x() - fa.right(), 15);
        assert_eq!(fa.x(), 4);
        assert_eq!(fb.right(), 996);
        assert_eq!((fa.y(), fa.bottom()), (4, 596));
    }

    #[test]
    fn huge_gap_falls_back_to_the_tile() {
        let area = r(0, 0, 10, 10);
        assert_eq!(frame_rect(area, area, 0, 100), area);
    }

    #[test]
    fn negative_values_are_treated_as_zero() {
        let tile = r(0, 0, 100, 100);
        assert_eq!(frame_rect(tile, tile, -5, -5), tile);
        assert_eq!(window_rect(tile, -3), tile);
    }

    #[test]
    fn window_is_inset_by_the_border() {
        assert_eq!(window_rect(r(8, 8, 100, 50), 3), r(11, 11, 94, 44));
        assert_eq!(window_rect(r(0, 0, 4, 4), 3), r(0, 0, 4, 4));
    }

    #[test]
    fn bounds_cover_all_rects() {
        assert_eq!(bounds_of([]), None);
        assert_eq!(
            bounds_of([r(0, 0, 10, 10), r(20, 5, 10, 10)]),
            Some(r(0, 0, 30, 15))
        );
    }
}
