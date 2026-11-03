//! Contrast の実行ファイル（nested プロトタイプ）。
//!
//! ポリシーは `StaticPolicy` を既定として使う。端末は環境変数 `CONTRAST_TERMINAL`、
//! 無ければ `foot`。

use std::io::IsTerminal;

use contrast_policy::StaticPolicy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,smithay=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    let terminal = std::env::var("CONTRAST_TERMINAL").unwrap_or_else(|_| "foot".to_string());
    let policy = StaticPolicy::new(terminal);
    contrast_compositor::run(Box::new(policy))
}
