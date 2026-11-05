//! `ConfigLoader` の実装 `CdlLoader`。
//!
//! 流れは、ファイルの有無 → リゾルバ → `Engine::load`（構文・名前解決・型検査・スキーマ適合・評価）→
//! `to_settings`（値木の変換）→ `Config`。どの段階の失敗も `ConfigDiagnostic` の一覧にして返し、落ちない。

use std::io::ErrorKind;
use std::path::Path;
use std::rc::Rc;

use cdl_host::{Engine, Limits, ModuleResolver};
use contrast_policy::{Config, ConfigDiagnostic, ConfigLoader, LoadOutcome, codes};

use crate::convert::to_settings;
use crate::diag::{convert_all, internal};
use crate::hook::CdlPolicy;
use crate::resolver::{FsResolver, Recording};
use crate::schema::contrast_schema;

/// CDL の設定ファイルを読む読み込み器。
#[derive(Debug, Clone, Copy, Default)]
pub struct CdlLoader {
    /// CDL の資源上限。
    limits: Limits,
}

impl CdlLoader {
    /// 既定の上限で作る。
    pub fn new() -> Self {
        Self::default()
    }

    /// 上限を指定して作る。
    pub fn with_limits(limits: Limits) -> Self {
        Self { limits }
    }
}

impl ConfigLoader for CdlLoader {
    fn name(&self) -> &'static str {
        "cdl"
    }

    fn load(&self, path: &Path) -> LoadOutcome {
        match std::fs::metadata(path) {
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => return LoadOutcome::Absent,
            Err(e) => {
                return LoadOutcome::Failed(vec![internal(
                    codes::FILE_UNREADABLE,
                    format!("{}: {e}", path.display()),
                )]);
            }
        }
        let (resolver, entry) =
            match FsResolver::for_entry(path, u64::from(self.limits.max_source_bytes)) {
                Ok(v) => v,
                Err(message) => {
                    return LoadOutcome::Failed(vec![internal(codes::FILE_UNREADABLE, message)]);
                }
            };
        match load_config(&resolver, &entry, self.limits) {
            Ok(config) => LoadOutcome::Loaded(config),
            Err(diags) => LoadOutcome::Failed(diags),
        }
    }
}

/// `resolver` からエントリ `entry` を読み、`Config` にする。
pub(crate) fn load_config(
    resolver: &dyn ModuleResolver,
    entry: &str,
    limits: Limits,
) -> Result<Config, Vec<ConfigDiagnostic>> {
    let schema = contrast_schema().map_err(|errors| {
        errors
            .into_iter()
            .map(|e| internal(codes::TREE_MISMATCH, format!("スキーマの登録に失敗: {e}")))
            .collect::<Vec<_>>()
    })?;
    let recording = Recording::new(resolver);
    let engine = Engine::new().with_schema(schema).with_limits(limits);
    let program = match engine.load(&recording, entry) {
        Ok(program) => program,
        Err(diags) => return Err(convert_all(&diags, &recording.into_sources())),
    };
    let sources = recording.into_sources();
    let settings = to_settings(program.root(), &sources)?;
    let hook = CdlPolicy::new(Rc::new(program), sources);
    Ok(Config::new(settings, Some(Box::new(hook))))
}
