//! 仕様テストの入口。人間だけが編集する。フェーズごとに mod を足していく。
//! 実行: cargo test -p contrast-core --test spec [フィルタ]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::missing_docs_in_private_items,
    missing_docs,
    reason = "仕様テストは失敗＝パニックでよい。本体の lint は緩めない"
)]
mod spec_geometry;
mod spec_id;
