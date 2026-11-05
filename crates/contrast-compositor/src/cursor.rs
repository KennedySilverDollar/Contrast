//! カーソルの絵: xcursor のテーマ（`XCURSOR_THEME` / `XCURSOR_SIZE`）、クライアントが `wl_pointer.set_cursor` で
//! 指定した面、テーマが無いときの色つきの四角。DRM バックエンドだけが使う（winit ではホストがカーソルを描く）。
//!
//! テーマの読み込みの失敗は落ちない（そのカーソルは「無い」ことにして、四角に戻る）。アニメーションは最初のコマだけ。

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                Kind,
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                solid::{SolidColorBuffer, SolidColorRenderElement},
                surface::render_elements_from_surface_tree,
            },
            gles::GlesRenderer,
        },
    },
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    utils::{Logical, Physical, Point, Transform},
    wayland::compositor::with_states,
};

use crate::render::SceneElement;

/// カーソルの既定の大きさ（`XCURSOR_SIZE` が無いとき）。
const DEFAULT_SIZE: u32 = 24;
/// 簡易カーソル（四角）の一辺と色。
const FALLBACK_SIZE: i32 = 12;
const FALLBACK_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// テーマから読んだカーソル 1 つ。
pub struct ThemeCursor {
    /// 画素（Argb8888）。
    buffer: MemoryRenderBuffer,
    /// ホットスポット（画像の左上から）。
    hotspot: (i32, i32),
}

/// xcursor のテーマ。名前ごとに 1 度だけ読む（読めなかった名前も覚える）。
pub struct CursorTheme {
    name: String,
    size: u32,
    cache: BTreeMap<String, Option<ThemeCursor>>,
}

impl CursorTheme {
    /// `XCURSOR_THEME`（無ければ `default`）と `XCURSOR_SIZE`（無ければ 24）で作る。まだ何も読まない。
    pub fn from_env() -> Self {
        let name = std::env::var("XCURSOR_THEME")
            .ok()
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "default".to_owned());
        let size = std::env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .filter(|s| (8..=256).contains(s))
            .unwrap_or(DEFAULT_SIZE);
        Self::new(name, size)
    }

    /// テーマ名と大きさを指定して作る。
    pub fn new(name: String, size: u32) -> Self {
        Self {
            name,
            size,
            cache: BTreeMap::new(),
        }
    }

    /// カーソル `icon`（xcursor の名前）を返す。テーマに無ければ `default`、それも無ければ `None`。
    fn get(&mut self, icon: &str) -> Option<&ThemeCursor> {
        let key = if self.load(icon) { icon } else { "default" };
        self.load(key);
        self.cache.get(key).and_then(Option::as_ref)
    }

    /// `icon` を（まだなら）読む。読めたかを返す。
    fn load(&mut self, icon: &str) -> bool {
        if !self.cache.contains_key(icon) {
            let cursor = load_cursor(&self.name, icon, self.size);
            if cursor.is_none() {
                tracing::debug!(theme = %self.name, icon, "cursor: not found in the theme");
            }
            self.cache.insert(icon.to_owned(), cursor);
        }
        self.cache.get(icon).is_some_and(Option::is_some)
    }
}

/// テーマ `theme` のカーソル `icon` を、大きさが `size` に最も近い画像で読む。
pub fn load_cursor(theme: &str, icon: &str, size: u32) -> Option<ThemeCursor> {
    let path: PathBuf = xcursor::CursorTheme::load(theme).load_icon(icon)?;
    let data = std::fs::read(path).ok()?;
    parse_cursor(&data, size)
}

/// xcursor ファイルの中身から、`size` に最も近い画像（最初のコマ）を取り出す。壊れていれば `None`。
pub fn parse_cursor(data: &[u8], size: u32) -> Option<ThemeCursor> {
    let images = xcursor::parser::parse_xcursor(data)?;
    let nominal = images.iter().min_by_key(|i| i.size.abs_diff(size))?.size;
    let image = images.iter().find(|i| i.size == nominal)?;
    let (w, h) = (
        i32::try_from(image.width).ok()?,
        i32::try_from(image.height).ok()?,
    );
    let expected = usize::try_from(w)
        .ok()?
        .checked_mul(usize::try_from(h).ok()?)?
        .checked_mul(4)?;
    if w <= 0 || h <= 0 || image.pixels_rgba.len() < expected {
        return None;
    }
    let buffer = MemoryRenderBuffer::from_slice(
        &image.pixels_rgba,
        Fourcc::Argb8888,
        (w, h),
        1,
        Transform::Normal,
        None,
    );
    Some(ThemeCursor {
        buffer,
        hotspot: (
            i32::try_from(image.xhot).ok()?,
            i32::try_from(image.yhot).ok()?,
        ),
    })
}

impl ThemeCursor {
    /// ホットスポット（画像の左上から）。
    pub fn hotspot(&self) -> (i32, i32) {
        self.hotspot
    }
}

/// いまのカーソルの描画要素（最前面に置く）。`pos` はポインタの論理座標。
pub fn cursor_elements(
    renderer: &mut GlesRenderer,
    theme: &mut CursorTheme,
    fallback: &mut SolidColorBuffer,
    status: &CursorImageStatus,
    pos: Point<f64, Logical>,
) -> Vec<SceneElement> {
    match status {
        CursorImageStatus::Hidden => Vec::new(),
        CursorImageStatus::Surface(surface) => {
            let hotspot = with_states(surface, |states| {
                states
                    .data_map
                    .get::<Mutex<CursorImageAttributes>>()
                    .and_then(|attrs| attrs.lock().ok().map(|a| a.hotspot))
            })
            .unwrap_or_default();
            let at: Point<i32, Physical> = (pos - hotspot.to_f64()).to_physical_precise_round(1);
            render_elements_from_surface_tree(renderer, surface, at, 1.0, 1.0, Kind::Cursor)
        }
        CursorImageStatus::Named(icon) => match theme.get(icon.name()) {
            Some(cursor) => {
                let (hx, hy) = cursor.hotspot();
                let at: Point<f64, Physical> =
                    (pos.x - f64::from(hx), pos.y - f64::from(hy)).into();
                match MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    at,
                    &cursor.buffer,
                    None,
                    None,
                    None,
                    Kind::Cursor,
                ) {
                    Ok(element) => vec![SceneElement::Memory(element)],
                    Err(e) => {
                        tracing::warn!("cursor: texture import failed: {e}");
                        vec![fallback_square(fallback, pos)]
                    }
                }
            }
            None => vec![fallback_square(fallback, pos)],
        },
    }
}

/// テーマが使えないときの、色つきの四角。
fn fallback_square(buffer: &mut SolidColorBuffer, pos: Point<f64, Logical>) -> SceneElement {
    buffer.update((FALLBACK_SIZE, FALLBACK_SIZE), FALLBACK_COLOR);
    let at: Point<i32, Logical> = pos.to_i32_round();
    SceneElement::Solid(SolidColorRenderElement::from_buffer(
        buffer,
        (at.x, at.y),
        1.0,
        1.0,
        Kind::Unspecified,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 大きさ `nominal` の 1x1 画像 1 つだけの xcursor ファイル。
    fn xcursor_file(nominal: u32, xhot: u32, yhot: u32) -> Vec<u8> {
        let mut v = Vec::new();
        let put = |v: &mut Vec<u8>, n: u32| v.extend_from_slice(&n.to_le_bytes());
        v.extend_from_slice(b"Xcur");
        put(&mut v, 16); // header size
        put(&mut v, 0x0001_0000); // version
        put(&mut v, 1); // ntoc
        put(&mut v, 0xfffd_0002); // toc: type = image
        put(&mut v, nominal); // toc: subtype
        put(&mut v, 28); // toc: position
        put(&mut v, 36); // chunk header size
        put(&mut v, 0xfffd_0002);
        put(&mut v, nominal);
        put(&mut v, 1); // chunk version
        put(&mut v, 1); // width
        put(&mut v, 1); // height
        put(&mut v, xhot);
        put(&mut v, yhot);
        put(&mut v, 0); // delay
        put(&mut v, 0xff11_2233); // pixel
        v
    }

    #[test]
    fn parses_a_cursor_and_keeps_the_hotspot() {
        let cursor = parse_cursor(&xcursor_file(24, 0, 0), 24);
        assert_eq!(cursor.map(|c| c.hotspot()), Some((0, 0)));
    }

    #[test]
    fn broken_data_is_none_not_a_panic() {
        assert!(parse_cursor(b"", 24).is_none());
        assert!(parse_cursor(b"not an xcursor file", 24).is_none());
        let mut truncated = xcursor_file(24, 0, 0);
        truncated.truncate(30);
        assert!(parse_cursor(&truncated, 24).is_none());
    }

    /// インストール済みのテーマ（`XCURSOR_THEME`、無ければ `default`）があれば、読めること。無ければ何もしない。
    #[test]
    fn installed_theme_loads_when_present() {
        let theme = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".to_owned());
        if let Some(cursor) = load_cursor(&theme, "default", 24) {
            let (hx, hy) = cursor.hotspot();
            eprintln!("cursor theme {theme}: hotspot=({hx}, {hy})");
            assert!(hx >= 0 && hy >= 0);
        }
    }

    #[test]
    fn missing_theme_gives_no_cursor() {
        assert!(load_cursor("contrast-no-such-theme-xyz", "default", 24).is_none());
        let mut theme = CursorTheme::new("contrast-no-such-theme-xyz".to_owned(), 24);
        assert!(theme.get("default").is_none());
        assert!(theme.get("text").is_none());
    }
}
