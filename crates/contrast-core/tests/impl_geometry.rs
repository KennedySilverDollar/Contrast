//! geometry の実装テスト: 境界値と、proptest による性質の確認。

/// `expect` を補助関数でも使えるよう、テスト一式を `cfg(test)` モジュールに入れる。
#[cfg(test)]
mod geometry {
    use contrast_core::{LayoutError, Point, Rect, Size};
    use proptest::prelude::*;

    /// テスト用に必ず作れる矩形を作る。
    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect::new(x, y, w, h).expect("テスト入力は有効な矩形")
    }

    /// `Size::new` は 0 と負を拒み、1 を受け付ける。
    #[test]
    fn size_lower_bound() {
        assert!(Size::new(0, 1).is_none());
        assert!(Size::new(1, 0).is_none());
        assert!(Size::new(-1, 5).is_none());
        let s = Size::new(1, 1).expect("1x1 は有効");
        assert_eq!((s.width(), s.height()), (1, 1));
    }

    /// `Rect::new` は右端・下端が i32 に収まる境界ちょうどまで受け付ける。
    #[test]
    fn rect_new_overflow_boundary() {
        assert!(Rect::new(i32::MAX - 1, 0, 1, 1).is_some());
        assert!(Rect::new(i32::MAX, 0, 1, 1).is_none());
        assert!(Rect::new(0, i32::MAX, 1, 1).is_none());
        assert!(Rect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX).is_some());
        assert!(Rect::new(0, 0, 0, 1).is_none());
    }

    /// アクセサと中心（切り捨て）。
    #[test]
    fn accessors_and_center() {
        let r = rect(10, 20, 5, 4);
        assert_eq!((r.x(), r.y(), r.width(), r.height()), (10, 20, 5, 4));
        assert_eq!((r.right(), r.bottom()), (15, 24));
        assert_eq!(r.center(), Point::new(12, 22));
        let big = rect(i32::MIN, i32::MIN, i32::MAX, i32::MAX);
        assert!(big.contains(big.center()));
    }

    /// `contains` は右端・下端を含まない半開区間。
    #[test]
    fn contains_half_open() {
        let r = rect(0, 0, 2, 2);
        assert!(r.contains(Point::new(0, 0)));
        assert!(r.contains(Point::new(1, 1)));
        assert!(!r.contains(Point::new(2, 1)));
        assert!(!r.contains(Point::new(1, 2)));
        assert!(!r.contains(Point::new(-1, 0)));
    }

    /// 接するだけの矩形の共通部分は `None`、重なれば重なり部分。
    #[test]
    fn intersect_examples() {
        let a = rect(0, 0, 10, 10);
        assert_eq!(a.intersect(&rect(10, 0, 5, 5)), None);
        assert_eq!(a.intersect(&rect(5, 5, 10, 10)), Some(rect(5, 5, 5, 5)));
        assert_eq!(a.intersect(&a), Some(a));
        let far = rect(i32::MIN, i32::MIN, 1, 1);
        let near_max = rect(i32::MAX - 1, i32::MAX - 1, 1, 1);
        assert_eq!(far.intersect(&near_max), None);
    }

    /// 余りは先頭から 1 ずつ配る。
    #[test]
    fn split_remainder_goes_first() {
        let cols = rect(0, 0, 10, 3)
            .split_columns(3)
            .expect("10 を 3 列に分けられる");
        let widths: Vec<i32> = cols.iter().map(Rect::width).collect();
        assert_eq!(widths, vec![4, 3, 3]);
        let xs: Vec<i32> = cols.iter().map(Rect::x).collect();
        assert_eq!(xs, vec![0, 4, 7]);
        let rows = rect(0, 5, 3, 7)
            .split_rows(2)
            .expect("7 を 2 行に分けられる");
        let heights: Vec<i32> = rows.iter().map(Rect::height).collect();
        assert_eq!(heights, vec![4, 3]);
        let ys: Vec<i32> = rows.iter().map(Rect::y).collect();
        assert_eq!(ys, vec![5, 9]);
    }

    /// n == 0 と長さ超過は `AreaTooSmall`、長さちょうどは幅 1 ずつ。
    #[test]
    fn split_too_small() {
        let r = rect(0, 0, 3, 2);
        assert_eq!(r.split_columns(0), Err(LayoutError::AreaTooSmall));
        assert_eq!(r.split_columns(4), Err(LayoutError::AreaTooSmall));
        assert_eq!(r.split_rows(3), Err(LayoutError::AreaTooSmall));
        assert_eq!(r.split_rows(usize::MAX), Err(LayoutError::AreaTooSmall));
        let ones = r.split_columns(3).expect("3 を 3 列に分けられる");
        assert!(ones.iter().all(|c| c.width() == 1));
    }

    /// 有効な矩形を生成する戦略。
    fn any_rect() -> impl Strategy<Value = Rect> {
        (any::<i32>(), any::<i32>(), 1..=i32::MAX, 1..=i32::MAX)
            .prop_filter_map("右端・下端が i32 に収まる", |(x, y, w, h)| {
                Rect::new(x, y, w, h)
            })
    }

    /// 小さめの座標で重なりやすい矩形を生成する戦略。
    fn small_rect() -> impl Strategy<Value = Rect> {
        (-50..50_i32, -50..50_i32, 1..60_i32, 1..60_i32)
            .prop_filter_map("常に有効", |(x, y, w, h)| Rect::new(x, y, w, h))
    }

    proptest! {
        /// intersect は対称。
        #[test]
        fn intersect_symmetric(a in any_rect(), b in any_rect()) {
            prop_assert_eq!(a.intersect(&b), b.intersect(&a));
        }

        /// 点が共通部分に含まれることと両方に含まれることは同値。
        #[test]
        fn intersect_contained_in_both(a in small_rect(), b in small_rect(),
                                       px in -60..120_i32, py in -60..120_i32) {
            let p = Point::new(px, py);
            let in_i = a.intersect(&b).is_some_and(|i| i.contains(p));
            prop_assert_eq!(in_i, a.contains(p) && b.contains(p));
        }

        /// 列分割は元の矩形を隙間・重なりなく覆い、各列の差は高々 1。
        #[test]
        fn split_columns_partitions(r in any_rect(), n in 1..64_usize) {
            let n_i32 = i32::try_from(n).expect("64 未満");
            let res = r.split_columns(n);
            if r.width() < n_i32 {
                prop_assert_eq!(res, Err(LayoutError::AreaTooSmall));
            } else {
                let cols = res.expect("幅が足りる");
                prop_assert_eq!(cols.len(), n);
                let mut x = r.x();
                for c in &cols {
                    prop_assert_eq!((c.x(), c.y(), c.height()), (x, r.y(), r.height()));
                    x = c.right();
                }
                prop_assert_eq!(x, r.right());
                let max = cols.iter().map(Rect::width).max().expect("n >= 1");
                let min = cols.iter().map(Rect::width).min().expect("n >= 1");
                prop_assert!(max.checked_sub(min).is_some_and(|d| d <= 1));
            }
        }

        /// 行分割は元の矩形を隙間・重なりなく覆い、各行の差は高々 1。
        #[test]
        fn split_rows_partitions(r in any_rect(), n in 1..64_usize) {
            let n_i32 = i32::try_from(n).expect("64 未満");
            let res = r.split_rows(n);
            if r.height() < n_i32 {
                prop_assert_eq!(res, Err(LayoutError::AreaTooSmall));
            } else {
                let rows = res.expect("高さが足りる");
                prop_assert_eq!(rows.len(), n);
                let mut y = r.y();
                for c in &rows {
                    prop_assert_eq!((c.x(), c.y(), c.width()), (r.x(), y, r.width()));
                    y = c.bottom();
                }
                prop_assert_eq!(y, r.bottom());
                let max = rows.iter().map(Rect::height).max().expect("n >= 1");
                let min = rows.iter().map(Rect::height).min().expect("n >= 1");
                prop_assert!(max.checked_sub(min).is_some_and(|d| d <= 1));
            }
        }
    }
}
