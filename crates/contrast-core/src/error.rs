//! core が返すエラー型。
//!
//! 文字列エラーは使わず、失敗の種類ごとに variant を持つ。

/// レイアウト計算（矩形の分割など）の失敗。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutError {
    /// 領域が小さすぎて、各要素に幅（高さ）1 以上を割り当てられない。
    AreaTooSmall,
}
