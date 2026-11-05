//! 見た目の設定（`Layout`・`Appearance` の角丸と影・`Cursor`）の変換のテスト: 値、既定、範囲外の診断。

use std::fs;
use std::path::{Path, PathBuf};

use contrast_policy::{ConfigLoader, LoadOutcome, Settings};
use contrast_policy_cdl::CdlLoader;

/// 一時ファイルを書いて、そのパスを返す。`body` は `<Contrast>` の中身。
fn write_temp(dir: &str, body: &str) -> Result<PathBuf, String> {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("impl_look")
        .join(dir);
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let path = base.join("config.cdl");
    let text = format!(
        "\"use cdl 1\";\nimport {{ Contrast, Appearance, Layout, Cursor }} from \"host:contrast\";\nexport default (\n  <Contrast>\n{body}\n  </Contrast>\n);\n"
    );
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path)
}

/// 読み込んで、設定か診断コードの並びを返す。
fn load(dir: &str, body: &str) -> Result<Result<Settings, Vec<String>>, String> {
    let path = write_temp(dir, body)?;
    Ok(match CdlLoader::new().load(&path) {
        LoadOutcome::Loaded(c) => Ok(c.settings),
        LoadOutcome::Failed(d) => Err(d.iter().map(|d| d.code.clone()).collect()),
        LoadOutcome::Absent => Err(vec!["absent".to_owned()]),
    })
}

#[test]
fn defaults_keep_the_current_look() -> Result<(), String> {
    let s = load("none", "")?.map_err(|d| format!("{d:?}"))?;
    let b = Settings::builtin();
    assert_eq!(s.layout, b.layout);
    assert_eq!(s.layout.gap, 0);
    assert_eq!(s.layout.outer_gap, 0);
    assert_eq!(s.appearance.radius, 0);
    assert_eq!(s.appearance.shadow.blur, 0);
    assert_eq!(s.appearance.inactive_border_width, None);
    assert_eq!(s.cursor.theme, None);
    assert_eq!(s.cursor.size, None);
    Ok(())
}

#[test]
fn layout_appearance_and_cursor_are_read() -> Result<(), String> {
    let body = r##"
    <Appearance borderWidth={px(4)} focusColor={color("#7fc8ff")} radius={px(12)} shadowBlur={px(30)}
      shadowOffsetX={px(0)} shadowOffsetY={px(-5)} shadowColor={color("#00000080")} inactiveBorderWidth={px(2)} />
    <Layout gap={px(16)} outerGap={px(8)} />
    <Cursor theme="Cecilia" size={24} />"##;
    let s = load("all", body)?.map_err(|d| format!("{d:?}"))?;
    assert_eq!(s.layout.gap, 16);
    assert_eq!(s.layout.outer_gap, 8);
    assert_eq!(s.appearance.radius, 12);
    assert_eq!(s.appearance.shadow.blur, 30);
    assert_eq!(s.appearance.shadow.offset_x, 0);
    assert_eq!(s.appearance.shadow.offset_y, -5);
    assert!((s.appearance.shadow.color[3] - 128.0 / 255.0).abs() < 1e-6);
    assert_eq!(s.appearance.inactive_border_width, Some(2));
    assert_eq!(s.cursor.theme.as_deref(), Some("Cecilia"));
    assert_eq!(s.cursor.size, Some(24));
    Ok(())
}

#[test]
fn out_of_range_values_are_ct003() -> Result<(), String> {
    let bad = [
        ("gap_neg", "<Layout gap={px(-1)} />"),
        ("gap_big", "<Layout gap={px(513)} />"),
        ("gap_frac", "<Layout outerGap={px(1.5)} />"),
        (
            "radius_neg",
            r##"<Appearance borderWidth={px(1)} focusColor={color("#fff")} radius={px(-2)} />"##,
        ),
        (
            "shadow_big",
            r##"<Appearance borderWidth={px(1)} focusColor={color("#fff")} shadowBlur={px(9999)} />"##,
        ),
        (
            "offset_big",
            r##"<Appearance borderWidth={px(1)} focusColor={color("#fff")} shadowOffsetX={px(-600)} />"##,
        ),
        ("size_small", r#"<Cursor size={4} />"#),
        ("size_big", r#"<Cursor size={1000} />"#),
        ("size_frac", r#"<Cursor size={24.5} />"#),
    ];
    for (dir, body) in bad {
        let r = load(dir, body)?;
        assert_eq!(r.err(), Some(vec!["CT003".to_owned()]), "{dir}");
    }
    Ok(())
}

#[test]
fn edges_of_the_ranges_are_accepted() -> Result<(), String> {
    let s = load(
        "edges",
        "<Layout gap={px(512)} outerGap={px(0)} /><Cursor size={256} />",
    )?
    .map_err(|d| format!("{d:?}"))?;
    assert_eq!(s.layout.gap, 512);
    assert_eq!(s.cursor.size, Some(256));
    let s = load("edges2", "<Cursor size={8} />")?.map_err(|d| format!("{d:?}"))?;
    assert_eq!(s.cursor.size, Some(8));
    Ok(())
}

#[test]
fn duplicate_layout_or_cursor_fails() -> Result<(), String> {
    assert!(load("dup1", "<Layout /><Layout />")?.is_err());
    assert!(load("dup2", "<Cursor /><Cursor />")?.is_err());
    Ok(())
}
