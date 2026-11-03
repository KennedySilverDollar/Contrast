//! Contrast コンポジタの中核ロジック。
//!
//! Wayland やレンダリングに依存しない、状態・レイアウト・フォーカスの純粋な実装。
//! 仕様は `docs/core-spec.md` を正とする。
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

/// エンティティと値オブジェクト。crate 内の他モジュールに依存しない。
mod domain {
    pub mod geometry;
}

mod error;

/// Kani ハーネス（`spec/` は人間、`extra/` は実装者）。
#[cfg(kani)]
mod verification;

pub use domain::geometry::{Point, Rect, Size};
pub use error::LayoutError;
