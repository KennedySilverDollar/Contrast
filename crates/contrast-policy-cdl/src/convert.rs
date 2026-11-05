//! 評価後の値木（`Contrast` 要素）を `Settings` にする。
//!
//! 誤りは見つけた順に全部集める。1 つでもあれば `Err` を返し、半端な `Settings` は作らない。
//! 値木の位置は要素単位（props には位置がない）なので、診断は誤りのある prop を含む要素の位置を指し、
//! メッセージに prop 名を書く。

use std::collections::BTreeMap;

use cdl_host::{Node, Px, Rgba};
use contrast_policy::settings::{MAX_BORDER_WIDTH, MAX_WORKSPACES, rgba8};
use contrast_policy::{Action, ConfigDiagnostic, KeyBinding, KeySpec, ModKey, Settings, codes};

use crate::diag::{at, convert};
use crate::keys::parse_key_spec;

/// 変換の文脈: 診断の行・列を求めるためのソース本文と、集めた診断。
struct Ctx<'a> {
    /// id → ソース本文。
    sources: &'a BTreeMap<String, String>,
    /// これまでに集めた診断。
    diags: Vec<ConfigDiagnostic>,
}

impl Ctx<'_> {
    /// `node` の位置に診断を足す。
    fn error(&mut self, node: Node<'_>, code: &str, message: impl Into<String>) {
        let span = &node.element().span;
        self.diags.push(at(span, code, message, self.sources));
    }

    /// 必須の prop を読む。読めなければ診断を足して `None`。
    fn prop<T: cdl_host::FromValue>(&mut self, node: Node<'_>, key: &str) -> Option<T> {
        match node.prop(key) {
            Ok(v) => Some(v),
            Err(d) => {
                self.diags.push(convert(&d, self.sources));
                None
            }
        }
    }

    /// 省略できる prop を読む。型が違えば診断を足して `None`。
    fn optional<T: cdl_host::FromValue>(&mut self, node: Node<'_>, key: &str) -> Option<T> {
        match node.optional_prop(key) {
            Ok(v) => v,
            Err(d) => {
                self.diags.push(convert(&d, self.sources));
                None
            }
        }
    }
}

/// 重複検査用のキー: (alt, shift, ctrl, super, キー名)。
type KeyId = (bool, bool, bool, bool, String);

/// `KeySpec` を重複検査用のキーにする。
fn key_id(k: &KeySpec) -> KeyId {
    (
        k.mods.alt,
        k.mods.shift,
        k.mods.ctrl,
        k.mods.logo,
        k.keysym.clone(),
    )
}

/// `n` が `lo..=hi` の整数ならその値。
fn integer_in(n: f64, lo: u32, hi: u32) -> Option<u32> {
    if !n.is_finite() || n.fract() != 0.0 || n < f64::from(lo) || n > f64::from(hi) {
        return None;
    }
    Some(n as u32)
}

/// 1 始まりのワークスペース番号 `n`（`1..=count`）を 0 始まりの `u8` にする。
fn workspace_index(n: f64, count: u8) -> Option<u8> {
    integer_in(n, 1, u32::from(count))
        .and_then(|v| v.checked_sub(1))
        .and_then(|v| u8::try_from(v).ok())
}

/// `Contrast` 要素を `Settings` にする。`sources` は診断の位置の計算に使う（id → 本文）。
pub(crate) fn to_settings(
    root: Node<'_>,
    sources: &BTreeMap<String, String>,
    default_mod_key: ModKey,
) -> Result<Settings, Vec<ConfigDiagnostic>> {
    let mut cx = Ctx {
        sources,
        diags: Vec::new(),
    };
    let mut settings = Settings::builtin();
    settings.keybinds.clear();

    if let Err(d) = root.expect("Contrast") {
        return Err(vec![convert(&d, sources)]);
    }
    settings.mod_key = read_mod_key(&mut cx, root, default_mod_key);

    // 1 回目: Appearance と Workspaces（キーバインドの検査に数が要る）。
    let mut keybind_nodes = Vec::new();
    for child in root.children() {
        match child.name() {
            "Appearance" => read_appearance(&mut cx, child, &mut settings),
            "Workspaces" => read_workspaces(&mut cx, child, &mut settings),
            "Input" => read_input(&mut cx, child, &mut settings),
            "Keybind" => keybind_nodes.push(child),
            other => cx.error(
                child,
                codes::TREE_MISMATCH,
                format!("未知の要素 `<{other}>`"),
            ),
        }
    }

    // 2 回目: Keybind。
    let mut seen: BTreeMap<KeyId, (u32, u32)> = BTreeMap::new();
    for node in keybind_nodes {
        let Some(binding) = read_keybind(&mut cx, node, &settings) else {
            continue;
        };
        let here = position(&cx, node);
        let id = key_id(&binding.key);
        if let Some((line, col)) = seen.get(&id) {
            cx.error(
                node,
                codes::DUPLICATE_KEY,
                format!(
                    "キー `{}` は {line}:{col} でも使っている",
                    prop_text(node, "keys")
                ),
            );
            continue;
        }
        seen.insert(id, here);
        settings.keybinds.push(binding);
    }

    if cx.diags.is_empty() {
        Ok(settings)
    } else {
        Err(cx.diags)
    }
}

/// 要素の行・列（求められなければ 0:0）。
fn position(cx: &Ctx<'_>, node: Node<'_>) -> (u32, u32) {
    let span = &node.element().span;
    cx.sources
        .get(&span.file)
        .and_then(|text| crate::diag::line_column(text, span.start))
        .unwrap_or((0, 0))
}

/// 文字列 prop の値（診断のメッセージ用。無ければ空）。
fn prop_text(node: Node<'_>, key: &str) -> String {
    node.optional_prop::<String>(key)
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// `Contrast` の `modKey`。省略したときは `default`（環境変数・バックエンドで決まった既定）。
fn read_mod_key(cx: &mut Ctx<'_>, root: Node<'_>, default: ModKey) -> ModKey {
    match cx.optional::<String>(root, "modKey").as_deref() {
        None => default,
        Some("alt") => ModKey::Alt,
        Some("super") => ModKey::Super,
        Some(other) => {
            cx.error(
                root,
                codes::BAD_PROP,
                format!("`modKey` は \"alt\" か \"super\"（\"{other}\" は使えない）"),
            );
            default
        }
    }
}

/// `Appearance` を読む。
fn read_appearance(cx: &mut Ctx<'_>, node: Node<'_>, settings: &mut Settings) {
    if let Some(Px(width)) = cx.prop::<Px>(node, "borderWidth") {
        let max = u32::try_from(MAX_BORDER_WIDTH).unwrap_or(0);
        match integer_in(width, 0, max).and_then(|w| i32::try_from(w).ok()) {
            Some(w) => settings.appearance.border_width = w,
            None => cx.error(
                node,
                codes::BAD_PROP,
                format!("`borderWidth` は 0〜{MAX_BORDER_WIDTH}px の整数（{width}px は使えない）"),
            ),
        }
    }
    if let Some(Rgba(c)) = cx.prop::<Rgba>(node, "focusColor") {
        settings.appearance.focus_color = rgba8(c);
    }
    if let Some(Rgba(c)) = cx.optional::<Rgba>(node, "borderColor") {
        settings.appearance.border_color = Some(rgba8(c));
    }
    if let Some(Rgba(c)) = cx.optional::<Rgba>(node, "background") {
        settings.appearance.background = rgba8(c);
    }
}

/// `Workspaces` を読む。
fn read_workspaces(cx: &mut Ctx<'_>, node: Node<'_>, settings: &mut Settings) {
    let Some(count) = cx.prop::<f64>(node, "count") else {
        return;
    };
    match integer_in(count, 1, u32::from(MAX_WORKSPACES)).and_then(|n| u8::try_from(n).ok()) {
        Some(n) => settings.workspaces = n,
        None => cx.error(
            node,
            codes::BAD_WORKSPACE_COUNT,
            format!("`count` は 1〜{MAX_WORKSPACES} の整数（{count} は使えない）"),
        ),
    }
}

/// `Input` を読む。省略した prop は既定のまま。`accelSpeed` は -1〜1 の数。
fn read_input(cx: &mut Ctx<'_>, node: Node<'_>, settings: &mut Settings) {
    if let Some(v) = cx.optional::<bool>(node, "tapToClick") {
        settings.input.tap_to_click = v;
    }
    if let Some(v) = cx.optional::<bool>(node, "naturalScroll") {
        settings.input.natural_scroll = v;
    }
    if let Some(v) = cx.optional::<bool>(node, "disableWhileTyping") {
        settings.input.disable_while_typing = v;
    }
    if let Some(speed) = cx.optional::<f64>(node, "accelSpeed") {
        if speed.is_finite() && (-1.0..=1.0).contains(&speed) {
            settings.input.accel_speed = speed;
        } else {
            cx.error(
                node,
                codes::BAD_PROP,
                format!("`accelSpeed` は -1〜1 の数（{speed} は使えない）"),
            );
        }
    }
}

/// `Keybind` を読む。誤りがあれば診断を足して `None`。
fn read_keybind(cx: &mut Ctx<'_>, node: Node<'_>, settings: &Settings) -> Option<KeyBinding> {
    let keys = cx.prop::<String>(node, "keys");
    let action = cx.prop::<String>(node, "action");
    let command = cx.optional::<String>(node, "command");
    let workspace = cx.optional::<f64>(node, "workspace");
    let (keys, action) = (keys?, action?);

    let spec: Option<KeySpec> = match parse_key_spec(&keys, settings.mod_key) {
        Ok(spec) => Some(spec),
        Err(e) => {
            cx.error(node, codes::BAD_KEY, format!("`keys` \"{keys}\": {e}"));
            None
        }
    };
    let action = read_action(cx, node, &action, command, workspace, settings.workspaces);
    Some(KeyBinding {
        key: spec?,
        action: action?,
    })
}

/// `action` の名前と `command`・`workspace` から `Action` を作る。
fn read_action(
    cx: &mut Ctx<'_>,
    node: Node<'_>,
    name: &str,
    command: Option<String>,
    workspace: Option<f64>,
    count: u8,
) -> Option<Action> {
    let simple = match name {
        "close" => Some(Action::CloseFocused),
        "focus-left" => Some(Action::FocusPrev),
        "focus-right" => Some(Action::FocusNext),
        "move-left" => Some(Action::MoveLeft),
        "move-right" => Some(Action::MoveRight),
        "toggle-floating" => Some(Action::ToggleFloating),
        "toggle-fullscreen" => Some(Action::ToggleFullscreen),
        "workspace-next" => Some(Action::WorkspaceNext),
        "workspace-prev" => Some(Action::WorkspacePrev),
        "move-to-workspace-next" => Some(Action::MoveToWorkspaceNext),
        "move-to-workspace-prev" => Some(Action::MoveToWorkspacePrev),
        "reload-config" => Some(Action::ReloadConfig),
        "quit" => Some(Action::Quit),
        _ => None,
    };
    if simple.is_some() {
        return simple;
    }
    match name {
        "spawn" => match command.map(|c| c.trim().to_owned()) {
            Some(c) if !c.is_empty() => Some(Action::Spawn(c)),
            _ => {
                cx.error(
                    node,
                    codes::MISSING_COMMAND,
                    "`spawn` には `command` が要る",
                );
                None
            }
        },
        "workspace" | "move-to-workspace" => {
            let Some(n) = workspace else {
                cx.error(
                    node,
                    codes::BAD_ACTION_WORKSPACE,
                    format!("`{name}` には `workspace` が要る"),
                );
                return None;
            };
            let Some(index) = workspace_index(n, count) else {
                cx.error(
                    node,
                    codes::BAD_ACTION_WORKSPACE,
                    format!("`workspace` は 1〜{count} の整数（{n} は使えない）"),
                );
                return None;
            };
            if name == "workspace" {
                Some(Action::SwitchWorkspace(index))
            } else {
                Some(Action::MoveToWorkspace(index))
            }
        }
        other => {
            cx.error(
                node,
                codes::UNKNOWN_ACTION,
                format!(
                    "未知の action \"{other}\"（spawn / close / focus-left / focus-right / move-left / move-right / \
                     workspace / move-to-workspace / workspace-next / workspace-prev / move-to-workspace-next / \
                     move-to-workspace-prev / toggle-floating / toggle-fullscreen / reload-config / quit）"
                ),
            );
            None
        }
    }
}
