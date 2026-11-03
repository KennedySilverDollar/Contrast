//! geometry の追加 Kani ハーネス（実装者が足したもの）。
//!
//! 仕様ハーネス（`verification/spec/geometry.rs`）を補い、構築・アクセサ・
//! `contains`・`intersect`・分割がパニックせず性質を満たすことを有界に証明する。

use crate::domain::geometry::{Point, Rect, Size};
use crate::error::LayoutError;

/// 任意の i32 4 つ組から `Rect` を作る。作れない入力は `kani::assume` で除く。
fn any_rect() -> Option<Rect> {
    let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
    let r = Rect::new(x, y, w, h);
    kani::assume(r.is_some());
    r
}

/// `Size::new` は幅・高さが 1 以上のときだけ `Some`。
#[kani::proof]
fn size_new_iff_positive() {
    let (w, h): (i32, i32) = kani::any();
    kani::assert(
        Size::new(w, h).is_some() == (w >= 1 && h >= 1),
        "Size::new の下限",
    );
}

/// `Rect::new` は Size の条件と右端・下端が i32 に収まるときだけ `Some` で、
/// 成功時のアクセサは入力と一致し、パニックしない。
#[kani::proof]
fn rect_new_and_accessors() {
    let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
    let ok = w >= 1 && h >= 1 && x.checked_add(w).is_some() && y.checked_add(h).is_some();
    let r = Rect::new(x, y, w, h);
    kani::assert(r.is_some() == ok, "Rect::new の成立条件");
    if let Some(r) = r {
        kani::assert(r.x() == x && r.y() == y, "原点");
        kani::assert(r.width() == w && r.height() == h, "大きさ");
        kani::assert(Some(r.right()) == x.checked_add(w), "右端");
        kani::assert(Some(r.bottom()) == y.checked_add(h), "下端");
        kani::assert(r.contains(r.center()), "中心は矩形内");
        kani::assert(r.contains(Point::new(x, y)), "左上は含む");
        kani::assert(!r.contains(Point::new(r.right(), y)), "右端は含まない");
        kani::assert(!r.contains(Point::new(x, r.bottom())), "下端は含まない");
    }
}

/// `intersect` は対称で、結果の各点は両方に含まれる（四隅で確認）。
#[kani::proof]
fn intersect_symmetric_and_contained() {
    let (Some(a), Some(b)) = (any_rect(), any_rect()) else {
        return;
    };
    let ab = a.intersect(&b);
    kani::assert(ab == b.intersect(&a), "intersect の対称性");
    if let Some(i) = ab {
        let corners = [
            Point::new(i.x(), i.y()),
            Point::new(i.right().saturating_sub(1), i.y()),
            Point::new(i.x(), i.bottom().saturating_sub(1)),
            Point::new(i.right().saturating_sub(1), i.bottom().saturating_sub(1)),
        ];
        for p in corners {
            kani::assert(a.contains(p) && b.contains(p), "共通部分は両方に含まれる");
        }
    }
}

/// 任意の点について、共通部分に含まれることと両方に含まれることは同値。
#[kani::proof]
fn intersect_contains_iff_both() {
    let (Some(a), Some(b)) = (any_rect(), any_rect()) else {
        return;
    };
    let p = Point::new(kani::any(), kani::any());
    let in_i = a.intersect(&b).is_some_and(|i| i.contains(p));
    kani::assert(in_i == (a.contains(p) && b.contains(p)), "共通部分の点集合");
}

/// `split_columns` / `split_rows` は n が 0 か長さ超過のとき `AreaTooSmall`、
/// それ以外は元の矩形を隙間・重なりなく n 個に覆い、各要素の差は高々 1。
#[kani::proof]
#[kani::unwind(6)]
fn split_partitions_or_too_small() {
    let Some(r) = any_rect() else {
        return;
    };
    let n: usize = kani::any();
    kani::assume(n <= 4);
    let cols = r.split_columns(n);
    let rows = r.split_rows(n);
    let n_i32 = i32::try_from(n).unwrap_or(i32::MAX);
    if n == 0 || r.width() < n_i32 {
        kani::assert(cols == Err(LayoutError::AreaTooSmall), "列分割の失敗条件");
    } else if let Ok(cols) = cols {
        kani::assert(cols.len() == n, "列数");
        let mut x = r.x();
        let first = cols.first().map_or(0, Rect::width);
        for c in &cols {
            kani::assert(
                c.x() == x && c.y() == r.y() && c.height() == r.height(),
                "隣接",
            );
            kani::assert(first.saturating_sub(c.width()) <= 1, "余りは先頭から");
            kani::assert(c.width() <= first, "幅は単調非増加");
            x = c.right();
        }
        kani::assert(x == r.right(), "右端まで覆う");
    } else {
        kani::assert(false, "列分割は成功するはず");
    }
    if n == 0 || r.height() < n_i32 {
        kani::assert(rows == Err(LayoutError::AreaTooSmall), "行分割の失敗条件");
    } else if let Ok(rows) = rows {
        kani::assert(rows.len() == n, "行数");
        let mut y = r.y();
        let first = rows.first().map_or(0, Rect::height);
        for c in &rows {
            kani::assert(
                c.y() == y && c.x() == r.x() && c.width() == r.width(),
                "隣接",
            );
            kani::assert(first.saturating_sub(c.height()) <= 1, "余りは先頭から");
            kani::assert(c.height() <= first, "高さは単調非増加");
            y = c.bottom();
        }
        kani::assert(y == r.bottom(), "下端まで覆う");
    } else {
        kani::assert(false, "行分割は成功するはず");
    }
}
