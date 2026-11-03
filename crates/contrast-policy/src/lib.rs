//! Contrast のポリシー層。
//!
//! - `port`: コンポジタがポリシーに問い合わせる差し込み口（トレイト）。
//! - `static_policy`: 静的な既定値による実装。今はこれを既定として使う。
//! - `core_adapter`: `port::Policy` を contrast-core の `Policy`（窓の配置）として使うアダプタ。
//! - `cdl_adapter`: CDL をあとから接続する場所（未実装）。

pub mod cdl_adapter;
pub mod core_adapter;
pub mod port;
pub mod static_policy;

pub use core_adapter::PortPlacement;
pub use port::{Action, Mods, Policy, WindowPlacement};
pub use static_policy::StaticPolicy;
