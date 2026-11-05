//! スクロール列レイアウトの値: `Ratio`、`ColumnWidth`、`LayoutMode`、`Column`、`LayoutSettings`。
//!
//! 守る不変条件:
//! - `Ratio` は `1 <= num <= den`（`new` が検査し、フィールドは非公開）。
//! - `ColumnWidth::fixed` は 1 以上だけを受ける。`Fixed` を直接作られても、幅の計算（`px`）は 1 以上に直す（I10）。
//! - `Column` は `Scrolling` のワークスペースだけが持つ。空の列を作らないのは `Workspace` の操作と `sync` の責任（I10）。

use alloc::vec::Vec;

use crate::domain::id::WindowId;

/// 比 `num / den`（`1 <= num <= den`）。列の幅を作業領域の幅の割合で表すのに使う。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ratio {
    /// 分子（1 以上、`den` 以下）。
    num: u32,
    /// 分母（`num` 以上）。
    den: u32,
}

impl Ratio {
    /// 1/2（既定の列幅）。
    pub(crate) const HALF: Self = Self { num: 1, den: 2 };
    /// 1/1（最大化の幅）。
    pub(crate) const FULL: Self = Self { num: 1, den: 1 };

    /// `1 <= num <= den` のときだけ `Some`。
    #[must_use]
    pub const fn new(num: u32, den: u32) -> Option<Self> {
        if num >= 1 && num <= den {
            Some(Self { num, den })
        } else {
            None
        }
    }

    /// 分子。
    #[must_use]
    pub const fn num(&self) -> u32 {
        self.num
    }

    /// 分母。
    #[must_use]
    pub const fn den(&self) -> u32 {
        self.den
    }
}

/// 列の幅。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnWidth {
    /// 作業領域の幅の割合（切り捨て、1 以上）。
    Proportion(Ratio),
    /// 固定の幅（px、1 以上）。
    Fixed(i32),
}

impl ColumnWidth {
    /// 固定幅。`px` が 1 未満なら `None`。
    #[must_use]
    pub const fn fixed(px: i32) -> Option<Self> {
        if px >= 1 { Some(Self::Fixed(px)) } else { None }
    }

    /// 作業領域の幅 `area_width` のときの px 幅（D37）。`Proportion` は切り捨て、どちらも 1 以上。
    #[must_use]
    pub fn px(self, area_width: i32) -> i32 {
        match self {
            Self::Fixed(px) => px.max(1),
            Self::Proportion(r) => {
                let wide = u128::from(area_width.max(0).unsigned_abs())
                    .saturating_mul(u128::from(r.num))
                    .checked_div(u128::from(r.den))
                    .unwrap_or(0);
                i32::try_from(wide).unwrap_or(i32::MAX).max(1)
            }
        }
    }

    /// 不正な `Fixed`（1 未満）を 1 に直す。
    #[must_use]
    pub(crate) const fn sanitized(self) -> Self {
        match self {
            Self::Fixed(px) if px < 1 => Self::Fixed(1),
            other => other,
        }
    }
}

/// 窓の並べ方。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LayoutMode {
    /// 既存の等分（`Layout` 型が決める）。
    #[default]
    Equal,
    /// スクロール列レイアウト（core-spec 12 節）。
    Scrolling,
}

/// プリセットの列幅の最大数。これより多く渡されたら、先頭のこの数だけを使う。
pub const MAX_PRESETS: usize = 8;

/// `Scrolling` の設定。`Core::new` が `CoreConfig` から作り、`State` が持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutSettings {
    /// 窓の並べ方。
    pub mode: LayoutMode,
    /// 新しい列の幅（D35。既定 1/2）。
    pub default_width: Ratio,
    /// プリセットの列幅（先頭から詰める。残りは `None`）。
    pub presets: [Option<Ratio>; MAX_PRESETS],
}

impl LayoutSettings {
    /// プリセットの既定: 1/3・1/2・2/3。
    pub(crate) const DEFAULT_PRESETS: [Option<Ratio>; MAX_PRESETS] = [
        Some(Ratio { num: 1, den: 3 }),
        Some(Ratio::HALF),
        Some(Ratio { num: 2, den: 3 }),
        None,
        None,
        None,
        None,
        None,
    ];

    /// 既定の設定（`Equal`、列幅 1/2、プリセット 1/3・1/2・2/3）。
    pub(crate) const DEFAULT: Self = Self {
        mode: LayoutMode::Equal,
        default_width: Ratio::HALF,
        presets: Self::DEFAULT_PRESETS,
    };

    /// 設定されているプリセットを順に返す。
    pub fn preset_iter(&self) -> impl Iterator<Item = Ratio> + '_ {
        self.presets.iter().flatten().copied()
    }
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 列 1 つ（D36）。窓は上から下の順。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Tiled の窓（上から下）。空にならない（I10）。
    pub windows: Vec<WindowId>,
    /// 幅。
    pub width: ColumnWidth,
    /// 最後にフォーカスした窓。`None` や列にない窓なら、先頭の窓を使う。
    pub last_focused: Option<WindowId>,
    /// 最大化する前の幅（最大化中だけ `Some`、D44）。
    pub restore_width: Option<ColumnWidth>,
}

impl Column {
    /// 窓 1 枚の新しい列。
    #[must_use]
    pub fn single(id: WindowId, width: ColumnWidth) -> Self {
        Self {
            windows: alloc::vec![id],
            width,
            last_focused: Some(id),
            restore_width: None,
        }
    }

    /// この列へフォーカスを移すときの窓: 最後にフォーカスした窓、なければ先頭の窓。
    #[must_use]
    pub fn focus_target(&self) -> Option<WindowId> {
        self.last_focused
            .filter(|id| self.windows.contains(id))
            .or_else(|| self.windows.first().copied())
    }
}
