//! ポリシーの差し込み口。
//!
//! コンポジタ（Smithay 側）はこのトレイトだけを見る。配置・キーバインド・見た目の
//! 決定はすべてここを経由させ、実装を `StaticPolicy` から CDL 由来の実装へ
//! 差し替えられるようにしておく。

use contrast_core::{Rect, WindowMeta, WindowState};

/// キー入力時の修飾キーの状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    /// Alt。
    pub alt: bool,
    /// Shift。
    pub shift: bool,
    /// Ctrl。
    pub ctrl: bool,
    /// Super / Logo。
    pub logo: bool,
}

/// キーバインド解決後の意味操作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// コマンドを起動する（引数なし、プログラム名のみ）。
    Spawn(String),
    /// フォーカス窓を閉じる。
    CloseFocused,
    /// 並びの前の窓へフォーカスを移す。
    FocusPrev,
    /// 並びの次の窓へフォーカスを移す。
    FocusNext,
    /// ワークスペースを切り替える（0 始まり）。
    SwitchWorkspace(u8),
    /// コンポジタを終了する。
    Quit,
}

/// 新しい窓の配置の判断（`Policy::place_window` の結果）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPlacement {
    /// 配置先のワークスペース（0 始まり）。`None` ならフォーカス出力の表示ワークスペース。
    pub workspace: Option<u8>,
    /// 初期状態。
    pub state: WindowState,
    /// 配置先が表示中ならフォーカスを与えるか。
    pub focus: bool,
}

/// コンポジタがポリシーに問い合わせる口。
pub trait Policy {
    /// 背景色（RGBA、0.0〜1.0）。
    fn background(&self) -> [f32; 4];

    /// フォーカス枠の太さ（論理ピクセル、0 で枠なし）。窓はタイルからこの幅だけ内側に縮めて置く。
    fn border_width(&self) -> i32;

    /// フォーカス枠の色（RGBA、0.0〜1.0）。
    fn focus_border_color(&self) -> [f32; 4];

    /// ワークスペースの数（1 以上）。
    fn workspace_count(&self) -> u8;

    /// 作業領域 `area` に `count` 個の窓を並べた矩形を、並び順に返す。
    ///
    /// 戻り値の長さは `count` と等しくなければならない。
    fn layout(&self, area: Rect, count: usize) -> Vec<Rect>;

    /// 新しい窓（app_id・title・is_dialog など）をどこにどう置くか決める。
    ///
    /// contrast-core の `Policy::place` には `core_adapter::PortPlacement` 経由で渡る。
    fn place_window(&self, meta: &WindowMeta) -> WindowPlacement;

    /// 修飾キーとキーシンボル名（xkb の名前、例 `"Return"`, `"q"`, `"Left"`）から
    /// 意味操作を引く。該当しなければ `None`（クライアントへ素通しする）。
    fn resolve_key(&self, mods: Mods, keysym: &str) -> Option<Action>;
}
