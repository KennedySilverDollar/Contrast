//! P1 仕様: 幾何。半開区間 [x, x+w) × [y, y+h)、座標は i32。
use contrast_core::{LayoutError, Point, Rect, Size};

fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect::new(x, y, w, h).expect("有効な矩形")
}

// ---- Size ----

#[test]
fn size_requires_positive_width_and_height() {
    assert!(Size::new(1, 1).is_some());
    assert!(Size::new(0, 5).is_none());
    assert!(Size::new(5, 0).is_none());
    assert!(Size::new(-1, 5).is_none());
    assert!(Size::new(5, -1).is_none());
}

#[test]
fn size_accessors() {
    let s = Size::new(7, 9).unwrap();
    assert_eq!((s.width(), s.height()), (7, 9));
}

// ---- Rect::new と右端・下端 ----

#[test]
fn rect_new_rejects_non_positive_size() {
    assert!(Rect::new(0, 0, 0, 1).is_none());
    assert!(Rect::new(0, 0, 1, 0).is_none());
    assert!(Rect::new(0, 0, -3, 4).is_none());
}

#[test]
fn rect_new_rejects_right_or_bottom_overflow() {
    assert!(Rect::new(i32::MAX, 0, 1, 1).is_none());
    assert!(Rect::new(1, 0, i32::MAX, 1).is_none());
    assert!(Rect::new(0, i32::MAX, 1, 1).is_none());
    assert!(Rect::new(0, 1, 1, i32::MAX).is_none());
}

#[test]
fn rect_new_accepts_edge_that_just_fits() {
    let r = Rect::new(i32::MAX - 1, 0, 1, 1).unwrap();
    assert_eq!(r.right(), i32::MAX);
    assert!(Rect::new(0, 0, i32::MAX, i32::MAX).is_some());
    let r = Rect::new(i32::MIN, 0, i32::MAX, 1).unwrap();
    assert_eq!(r.right(), -1);
}

#[test]
fn rect_accessors_and_edges() {
    let r = rect(10, 20, 30, 40);
    assert_eq!((r.x(), r.y(), r.width(), r.height()), (10, 20, 30, 40));
    assert_eq!((r.right(), r.bottom()), (40, 60));
}

#[test]
fn rect_center_rounds_toward_origin_side() {
    assert_eq!(rect(10, 20, 30, 40).center(), Point::new(25, 40));
    // 奇数の幅・高さは x + w/2, y + h/2（整数除算）
    assert_eq!(rect(0, 0, 5, 3).center(), Point::new(2, 1));
}

// ---- contains（半開区間） ----

#[test]
fn contains_is_half_open() {
    let r = rect(0, 0, 10, 10);
    assert!(r.contains(Point::new(0, 0)));
    assert!(r.contains(Point::new(9, 9)));
    assert!(!r.contains(Point::new(10, 5)));
    assert!(!r.contains(Point::new(5, 10)));
    assert!(!r.contains(Point::new(-1, 0)));
    assert!(!r.contains(Point::new(0, -1)));
}

// ---- intersect ----

#[test]
fn intersect_overlapping() {
    let a = rect(0, 0, 10, 10);
    let b = rect(5, 5, 10, 10);
    assert_eq!(a.intersect(&b), Some(rect(5, 5, 5, 5)));
    assert_eq!(b.intersect(&a), Some(rect(5, 5, 5, 5)));
}

#[test]
fn intersect_contained_returns_inner() {
    let a = rect(0, 0, 10, 10);
    let b = rect(2, 2, 3, 3);
    assert_eq!(a.intersect(&b), Some(b));
}

#[test]
fn intersect_touching_edges_is_none() {
    let a = rect(0, 0, 10, 10);
    assert_eq!(a.intersect(&rect(10, 0, 5, 5)), None);
    assert_eq!(a.intersect(&rect(0, 10, 5, 5)), None);
}

#[test]
fn intersect_disjoint_is_none() {
    assert_eq!(rect(0, 0, 10, 10).intersect(&rect(20, 20, 1, 1)), None);
}

// ---- split_columns / split_rows ----

fn spans_x(parts: &[Rect]) -> Vec<(i32, i32)> {
    parts.iter().map(|p| (p.x(), p.width())).collect()
}
fn spans_y(parts: &[Rect]) -> Vec<(i32, i32)> {
    parts.iter().map(|p| (p.y(), p.height())).collect()
}

#[test]
fn split_columns_gives_remainder_to_the_front() {
    let parts = rect(0, 0, 10, 5).split_columns(3).unwrap();
    assert_eq!(spans_x(&parts), vec![(0, 4), (4, 3), (7, 3)]);
    assert!(parts.iter().all(|p| p.y() == 0 && p.height() == 5));

    let parts = rect(0, 0, 10, 5).split_columns(4).unwrap();
    assert_eq!(spans_x(&parts), vec![(0, 3), (3, 3), (6, 2), (8, 2)]);
}

#[test]
fn split_columns_one_returns_same_rect() {
    let r = rect(3, 4, 10, 5);
    assert_eq!(r.split_columns(1).unwrap(), vec![r]);
}

#[test]
fn split_columns_with_negative_origin() {
    let parts = rect(-5, 0, 10, 2).split_columns(2).unwrap();
    assert_eq!(spans_x(&parts), vec![(-5, 5), (0, 5)]);
}

#[test]
fn split_columns_exact_fit_gives_one_pixel_each() {
    let parts = rect(0, 0, 3, 1).split_columns(3).unwrap();
    assert_eq!(spans_x(&parts), vec![(0, 1), (1, 1), (2, 1)]);
}

#[test]
fn split_columns_errors() {
    assert_eq!(
        rect(0, 0, 10, 5).split_columns(0),
        Err(LayoutError::AreaTooSmall)
    );
    assert_eq!(
        rect(0, 0, 2, 5).split_columns(3),
        Err(LayoutError::AreaTooSmall)
    );
}

#[test]
fn split_rows_gives_remainder_to_the_top() {
    let parts = rect(0, 0, 5, 10).split_rows(3).unwrap();
    assert_eq!(spans_y(&parts), vec![(0, 4), (4, 3), (7, 3)]);
    assert!(parts.iter().all(|p| p.x() == 0 && p.width() == 5));
}

#[test]
fn split_rows_errors() {
    assert_eq!(
        rect(0, 0, 5, 10).split_rows(0),
        Err(LayoutError::AreaTooSmall)
    );
    assert_eq!(
        rect(0, 0, 5, 2).split_rows(3),
        Err(LayoutError::AreaTooSmall)
    );
}
