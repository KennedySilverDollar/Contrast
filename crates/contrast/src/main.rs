//! Contrast の実行ファイル（nested プロトタイプ）。
//!
//! 設定ファイルのパスは `CONTRAST_CONFIG`、無ければ `~/.config/contrast/config.cdl`。
//! 読み込み器はまだスタブなので、ファイルの有無にかかわらず `StaticPolicy` で動く。
//! 端末は環境変数 `CONTRAST_TERMINAL`、無ければ `foot`。

use std::io::IsTerminal;

use contrast_policy::{
    ConfigLoader, LoadOutcome, NotImplementedLoader, Policy, StaticPolicy, resolve_config_path,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,smithay=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    let terminal = std::env::var("CONTRAST_TERMINAL").unwrap_or_else(|_| "foot".to_string());
    let policy = load_policy(&NotImplementedLoader, StaticPolicy::new(terminal));
    contrast_compositor::run(policy)
}

/// 設定ファイルを `loader` で読み、読めなければ `fallback` を使う。どの場合も落ちない。
fn load_policy(loader: &dyn ConfigLoader, fallback: StaticPolicy) -> Box<dyn Policy> {
    let path = resolve_config_path(
        std::env::var_os(contrast_policy::config::CONFIG_ENV),
        std::env::var_os("HOME"),
    );
    let Some(path) = path else {
        tracing::info!("config: path=<none> exists=false loader={}", loader.name());
        return Box::new(fallback);
    };
    let exists = path.exists();
    tracing::info!(
        "config: path={} exists={} loader={}",
        path.display(),
        exists,
        loader.name()
    );
    if !exists {
        return Box::new(fallback);
    }
    match loader.load(&path) {
        LoadOutcome::Loaded(policy) => {
            tracing::info!(path = %path.display(), "loaded config");
            policy
        }
        LoadOutcome::NotLoaded => Box::new(fallback),
        LoadOutcome::Failed(err) => {
            tracing::error!(path = %path.display(), %err, "config failed, using static policy");
            Box::new(fallback)
        }
    }
}
