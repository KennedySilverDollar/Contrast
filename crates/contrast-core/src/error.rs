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

/// core の公開操作の失敗。v0.1 で許す variant は仕様 6 節の 8 つだけ。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// 登録されていない `ExternalKey` への操作。
    UnknownKey,
    /// 登録済みの `ExternalKey`（または ID）の再登録。
    DuplicateKey,
    /// 出力追加時に未表示のワークスペースが残っていない。
    NoFreeWorkspace,
    /// `WorkspaceIndex` が `workspace_count` 以上。
    WorkspaceIndexOutOfRange,
    /// 対象窓が必要な Action でフォーカスが None。
    NoFocusedWindow,
    /// ID の上限到達。
    IdExhausted,
    /// 幾何計算が i32 に収まらない。
    GeometryOverflow,
    /// レイアウト計算の失敗。
    Layout(LayoutError),
}

impl core::fmt::Display for CoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::UnknownKey => "登録されていない ExternalKey への操作",
            Self::DuplicateKey => "ExternalKey または ID が登録済み",
            Self::NoFreeWorkspace => "未表示のワークスペースが残っていない",
            Self::WorkspaceIndexOutOfRange => "ワークスペースのインデックスが範囲外",
            Self::NoFocusedWindow => "フォーカスされた窓がない",
            Self::IdExhausted => "ID の上限に達した",
            Self::GeometryOverflow => "幾何計算が i32 に収まらない",
            Self::Layout(_) => "レイアウト計算の失敗",
        };
        f.write_str(text)
    }
}

impl core::error::Error for CoreError {}

impl From<LayoutError> for CoreError {
    fn from(e: LayoutError) -> Self {
        Self::Layout(e)
    }
}
