//! Contrast のスキーマ（`host:contrast`）を `cdl-host` のビルダーで登録する。内容は `docs/cdl-contract.md` の 1・2 節。
//!
//! `modKey` と `action` は本来リテラルユニオンだが、`cdl-host` が `Literal` を公開していないので
//! `string` にして、`convert` で検査する。

use cdl_host::{ComponentDef, Field, Schema, Ty};

/// ホストモジュール名。
const MODULE: &str = "host:contrast";

/// スキーマを組み立てる。失敗は `Schema::build` が返す誤りの一覧（実装の不具合を意味する）。
pub(crate) fn contrast_schema() -> Result<Schema, Vec<String>> {
    let window_info = Ty::object([
        ("appId", Field::required(Ty::String)),
        ("title", Field::optional(Ty::String)),
        ("isDialog", Field::required(Ty::Boolean)),
        ("workspace", Field::required(Ty::Number)),
    ]);
    let placement = Ty::object([
        ("workspace", Field::optional(Ty::Number)),
        ("floating", Field::required(Ty::Boolean)),
        ("focus", Field::optional(Ty::Boolean)),
    ]);
    Schema::builder()
        .root(MODULE, "Contrast")
        .component(
            MODULE,
            ComponentDef::new("Contrast")
                .optional_prop("modKey", Ty::String)
                .children(&["Appearance", "Workspaces", "Input", "Keybind"])
                .unique_child("Appearance")
                .unique_child("Workspaces")
                .unique_child("Input"),
        )
        .component(
            MODULE,
            ComponentDef::new("Appearance")
                .prop("borderWidth", Ty::Length)
                .prop("focusColor", Ty::Color)
                .optional_prop("borderColor", Ty::Color)
                .optional_prop("background", Ty::Color),
        )
        .component(
            MODULE,
            ComponentDef::new("Workspaces").prop("count", Ty::Number),
        )
        .component(
            MODULE,
            ComponentDef::new("Input")
                .optional_prop("tapToClick", Ty::Boolean)
                .optional_prop("naturalScroll", Ty::Boolean)
                .optional_prop("accelSpeed", Ty::Number)
                .optional_prop("disableWhileTyping", Ty::Boolean),
        )
        .component(
            MODULE,
            ComponentDef::new("Keybind")
                .prop("keys", Ty::String)
                .prop("action", Ty::String)
                .optional_prop("command", Ty::String)
                .optional_prop("workspace", Ty::Number),
        )
        .type_def(MODULE, "WindowInfo", window_info.clone())
        .type_def(MODULE, "Placement", placement.clone())
        .type_def(
            MODULE,
            "Policy",
            Ty::object([(
                "windowOpen",
                Field::optional(Ty::Function(vec![window_info], Box::new(placement))),
            )]),
        )
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_builds() {
        assert!(contrast_schema().is_ok());
    }
}
