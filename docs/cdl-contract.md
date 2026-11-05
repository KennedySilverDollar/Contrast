# Contrast ⇔ CDL 契約

Contrast が設定ファイル（CDL）に約束する内容。実装は `crates/contrast-policy-cdl`（CDL に触れるのはここだけ）と
`crates/contrast-policy`（CDL を知らない設定の型とストア）。CDL の API は `cdl-host` の公開 API だけを使う。
CDL 側の仕様は CDL リポジトリの `docs/SPEC.md`（6・8・9 章）と `docs/DECISIONS.md`（E1〜E8）。

## 1. スキーマ（モジュール `host:contrast`）

実装済みの機能だけを置く。ルート要素は `Contrast`。

| 要素 | props | 子 | 一意 |
| --- | --- | --- | --- |
| `Contrast` | `modKey?: "alt" \| "super"`（既定 `"alt"`） | `Appearance` \| `Workspaces` \| `Keybind` | `Appearance`、`Workspaces` |
| `Appearance` | `borderWidth: Length`、`focusColor: Color`、`borderColor?: Color`、`background?: Color` | なし | — |
| `Workspaces` | `count: number`（整数、1〜32） | なし | — |
| `Keybind` | `keys: string`、`action: string`、`command?: string`、`workspace?: number` | なし | — |

- `Appearance` と `Workspaces` を省略したときは `Settings::builtin()` の値を使う。
- `modKey` と `action` は本来リテラルユニオン（`"alt" | "super"` など）だが、`cdl-host` は `Literal` を公開していないので
  スキーマ上は `string` にして、Contrast の変換で検査する（誤りは CT003 / CT020）。
- `borderWidth` は 0 以上の整数（px。負・非整数は CT003）。`focusColor` はフォーカス窓の枠、`borderColor` は省略時は描かないフォーカス外の窓の枠、`background` は背景色（省略時は組み込みの色）。

### action

`spawn` / `close` / `focus-left` / `focus-right` / `move-left` / `move-right` / `workspace` / `move-to-workspace` /
`toggle-floating` / `toggle-fullscreen` / `reload-config` / `quit`。

- `spawn` は `command` 必須（空白で区切った先頭がプログラム、残りが引数）。
- `workspace` と `move-to-workspace` は `workspace` 必須。CDL では 1 始まり。core の `WorkspaceIndex` は 0 始まりなので
  変換する。1 未満・非整数・`Workspaces.count` 超過は CT022。
- 上記以外の action は CT020。`command` が要る action に無ければ CT021。

### キー書式

`mod+Return`、`mod+shift+q` のように `+` で区切る。最後が xkb のキー名、それ以前が修飾キー。

- 修飾キー: `mod`（`modKey` に従って alt か super）/ `alt` / `shift` / `ctrl` / `super`。大文字小文字は区別しない。
- キー名: xkb のキー名（`Return`、`q`、`Left`、`1`）。大文字小文字は区別しない。
- 修飾キーの重複（`alt+mod+x` で `mod` が alt など）は 1 つにまとめる。
- 空・キー名なし・未知の修飾キーは CT030。同じ（修飾キーの集合, キー名）の重複は CT031（2 つ目を指す）。

## 2. ポリシーのフック

```ts
interface WindowInfo { appId: string; title?: string; isDialog: boolean; workspace: number }
interface Placement  { workspace?: number; floating: boolean; focus?: boolean }
interface Policy     { windowOpen?: (w: WindowInfo) => Placement }
```

- `windowOpen(w)` は新しい窓が最初に map されるときに 1 回呼ばれる。
- `appId`: 無いときは `""`。`title`: 無いときは省略。`workspace`: フォーカス中のワークスペース（1 始まり）。
- 戻り値 `workspace` を省略したときはフォーカス中のワークスペース。`focus` を省略したときは `true`。
- フックを定義しないときは既定の配置（ダイアログは Floating、他は Tiled。フォーカスする）。

## 3. 失敗時の既定

| 状況 | 動作 |
| --- | --- |
| 起動時に読み込み・評価・変換のどれかが失敗 | `Settings::builtin()` と既定の配置で起動し、診断を error でログに出す。 |
| 設定ファイルがない | 既定値で起動する。エラーではない（info ログ）。 |
| 変換の誤りが 1 つでもある | その版を採用しない（半端な設定を作らない）。誤りは全部集めて報告する。 |
| リロードが失敗 | 旧版を保持し、診断を出す。世代は進めない。 |
| フックの上限超過・評価エラー・戻り値が不正（workspace が非整数・1 未満・範囲外など） | その呼び出しだけ既定の配置にし、診断を出す。同じコードは世代ごとに 1 回だけログに出す。 |
| リロードで `Workspaces.count` が変わった | core は起動後にワークスペース数を変えられない。新しい版は採用するが、数は起動時のまま。warning を出す（CT050）。 |

## 4. 診断コード（名前空間 CT。CDL の `E` とは別）

| コード | 意味 |
| --- | --- |
| CT001 | エントリの設定ファイルの状態を調べられない・パスを正準化できない（読めない・UTF-8 でない・サイズ超過は CDL の `E2001` で出る） |
| CT002 | 値木が期待と食い違う（未知の要素・ルートが違う。スキーマとの不一致で、通常は起きない） |
| CT003 | prop の値が不正（`modKey` が alt/super でない、`borderWidth` が負または非整数、など） |
| CT010 | `Workspaces.count` が 1〜32 の整数でない |
| CT020 | 未知の action |
| CT021 | `command` が要る action に `command` がない（または空） |
| CT022 | `workspace` が要る action に `workspace` がない・1 未満・非整数・`count` 超過 |
| CT030 | キー書式の誤り |
| CT031 | 同じキーの重複 |
| CT040 | フックの評価エラー・上限超過 |
| CT041 | フックの戻り値の形が不正 |
| CT042 | フックの戻り値の `workspace` が不正（非整数・1 未満） |
| CT043 | フックの戻り値の `workspace` が `Workspaces.count` を超える |
| CT050 | リロードで反映できない変更（ワークスペース数） |

CDL 自身の診断（`E0001`〜）は、そのままのコードで `ConfigDiagnostic` に写す。位置は `file:line:col`。
値木の診断は要素単位の位置しか取れない（props に位置がない）ので、誤りのある prop を含む要素の位置を示し、メッセージに prop 名を書く。

## 5. 設定ファイルの場所

`CONTRAST_CONFIG` → `$XDG_CONFIG_HOME/contrast/config.cdl` → `~/.config/contrast/config.cdl`（空の環境変数は無いものとして扱う）。
`import "./x.cdl"` は、設定ファイルのあるディレクトリの外を拒否する（`canonicalize` の後で判定するので、symlink で出るものも拒否）。
読む前にファイルのサイズを確認し、`Limits::max_source_bytes` を超えるものは読まない。

## 6. 未実装

- 識別 prop（意味が不明なので実装しない）。
- ファイル監視、画面上の診断表示、アニメーション、gap・radius。
