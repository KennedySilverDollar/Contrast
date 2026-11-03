//! Contrast のポリシー層。
//!
//! - `port`: コンポジタがポリシーに問い合わせる差し込み口（トレイト）。
//! - `static_policy`: 静的な既定値による実装。今はこれを既定として使う。
//! - `cdl_adapter`: CDL をあとから接続する場所（未実装）。

pub mod cdl_adapter;
pub mod port;
pub mod static_policy;

pub use port::{Action, Mods, Policy};
pub use static_policy::StaticPolicy;
