//! P1 の Kani 仕様ハーネス。全域の i32 入力でパニックなし、性質が成り立つことを示す。
use crate::domain::geometry::{Point, Rect};

/// Rect::new は任意の i32 でパニックせず、Some なら右端・下端が i32 に収まる。
#[kani::proof]
fn rect_new_never_panics_and_edges_fit() {
    let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
    if let Some(r) = Rect::new(x, y, w, h) {
        kani::assert(r.width() >= 1 && r.height() >= 1, "幅と高さは 1 以上");
        kani::assert(
            i64::from(r.right()) == i64::from(x) + i64::from(w),
            "right = x + width",
        );
        kani::assert(
            i64::from(r.bottom()) == i64::from(y) + i64::from(h),
            "bottom = y + height",
        );
    }
}

/// intersect は対称で、結果は両方の矩形に含まれる。
#[kani::proof]
fn intersect_symmetric_and_contained() {
    let (ax, ay, aw, ah): (i32, i32, i32, i32) = kani::any();
    let (bx, by, bw, bh): (i32, i32, i32, i32) = kani::any();
    let (Some(a), Some(b)) = (Rect::new(ax, ay, aw, ah), Rect::new(bx, by, bw, bh)) else {
        return;
    };
    let ab = a.intersect(&b);
    kani::assert(ab == b.intersect(&a), "対称");
    if let Some(i) = ab {
        kani::assert(i.x() >= a.x() && i.x() >= b.x(), "左端が両方の内側");
        kani::assert(i.y() >= a.y() && i.y() >= b.y(), "上端が両方の内側");
        kani::assert(i.right() <= a.right() && i.right() <= b.right(), "右端が両方の内側");
        kani::assert(i.bottom() <= a.bottom() && i.bottom() <= b.bottom(), "下端が両方の内側");
    }
}

/// 両方に含まれる点があれば、intersect は Some でその点を含む。
#[kani::proof]
fn intersect_covers_common_points() {
    let (ax, ay, aw, ah): (i32, i32, i32, i32) = kani::any();
    let (bx, by, bw, bh): (i32, i32, i32, i32) = kani::any();
    let (px, py): (i32, i32) = kani::any();
    let (Some(a), Some(b)) = (Rect::new(ax, ay, aw, ah), Rect::new(bx, by, bw, bh)) else {
        return;
    };
    let p = Point::new(px, py);
    if a.contains(p) && b.contains(p) {
        match a.intersect(&b) {
            Some(i) => kani::assert(i.contains(p), "共通点は交差矩形に含まれる"),
            None => kani::assert(false, "共通点があるのに None"),
        }
    }
}

/// split_columns は元の矩形を隙間・重なりなく n 列に分け、幅の差は高々 1。
#[kani::proof]
#[kani::unwind(6)]
fn split_columns_partitions_rect() {
    let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
    let n: usize = kani::any();
    kani::assume((1..=4).contains(&n));
    let Some(r) = Rect::new(x, y, w, h) else { return };
    match r.split_columns(n) {
        Err(_) => kani::assert(r.width() < n as i32, "失敗は幅不足のときだけ"),
        Ok(parts) => {
            kani::assert(parts.len() == n, "n 個に分かれる");
            let q = r.width() / n as i32;
            let mut cursor = r.x();
            for p in parts.iter() {
                kani::assert(p.x() == cursor, "隙間なく隣接する");
                kani::assert(p.y() == r.y() && p.height() == r.height(), "高さは元と同じ");
                kani::assert(p.width() >= q && p.width() - q <= 1, "幅は q か q+1");
                cursor = p.right();
            }
            kani::assert(cursor == r.right(), "右端が一致する");
        }
    }
}

/// split_rows は split_columns の縦版。
#[kani::proof]
#[kani::unwind(6)]
fn split_rows_partitions_rect() {
    let (x, y, w, h): (i32, i32, i32, i32) = kani::any();
    let n: usize = kani::any();
    kani::assume((1..=4).contains(&n));
    let Some(r) = Rect::new(x, y, w, h) else { return };
    match r.split_rows(n) {
        Err(_) => kani::assert(r.height() < n as i32, "失敗は高さ不足のときだけ"),
        Ok(parts) => {
            kani::assert(parts.len() == n, "n 個に分かれる");
            let q = r.height() / n as i32;
            let mut cursor = r.y();
            for p in parts.iter() {
                kani::assert(p.y() == cursor, "隙間なく隣接する");
                kani::assert(p.x() == r.x() && p.width() == r.width(), "幅は元と同じ");
                kani::assert(p.height() >= q && p.height() - q <= 1, "高さは q か q+1");
                cursor = p.bottom();
            }
            kani::assert(cursor == r.bottom(), "下端が一致する");
        }
    }
}
