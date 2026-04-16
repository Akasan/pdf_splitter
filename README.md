# pdfsplit

指定したPDFファイルのページを分割して保存するRust製CLIツール。

## 機能

- **ページ分割保存** — 1ページ = 1ファイルとして出力
  - 全ページ分割
  - 指定範囲のページを分割 (例: `3-7`)
  - 特定ページのみ抽出 (例: `1,3,5`)
  - 範囲と単一ページの混在指定 (例: `1,3-5,8`)
- **ページ内容の分割** — 1ページを物理的に分割して別ファイルとして保存
  - 左右分割 (horizontal) — `_left.pdf` / `_right.pdf`
  - 上下分割 (vertical) — `_top.pdf` / `_bottom.pdf`

内容分割は `MediaBox` / `CropBox` を書き換えて半分の領域を切り出す方式で、元の描画ストリームは変更せず軽量・高速に処理します。

## インストール

Rust (1.70+) が必要です。

```bash
cargo build --release
# バイナリ: target/release/pdfsplit
```

システムに配置する場合:

```bash
cargo install --path .
```

## 使い方

```
pdfsplit <INPUT> -o <OUTPUT_DIR> [OPTIONS]
```

### 引数・オプション

| 項目 | 説明 | デフォルト |
|---|---|---|
| `<INPUT>` | 入力PDFファイル | 必須 |
| `-o, --output-dir <DIR>` | 出力先ディレクトリ (なければ作成) | 必須 |
| `-p, --pages <SPEC>` | 対象ページ指定 (`all` / `3-7` / `1,3,5` / `1,3-5,8`) | `all` |
| `-s, --split <MODE>` | 内容分割モード (`none` / `horizontal` / `vertical`) | `none` |
| `--prefix <NAME>` | 出力ファイル名のプレフィックス | `page` |

### 出力ファイル名

- 分割なし: `<prefix>_<NNN>.pdf`
- 左右分割: `<prefix>_<NNN>_left.pdf` / `<prefix>_<NNN>_right.pdf`
- 上下分割: `<prefix>_<NNN>_top.pdf` / `<prefix>_<NNN>_bottom.pdf`

`<NNN>` は総ページ数に応じて0埋めされます (最低3桁)。

## 使用例

全ページを個別ファイルに分割:

```bash
pdfsplit input.pdf -o ./out
```

3〜7ページのみを分割:

```bash
pdfsplit input.pdf -o ./out -p 3-7
```

1・3・5ページだけを抽出:

```bash
pdfsplit input.pdf -o ./out -p 1,3,5
```

全ページを左右に分割して保存 (見開きPDFの単ページ化など):

```bash
pdfsplit input.pdf -o ./out -s horizontal
```

特定ページを上下に分割:

```bash
pdfsplit input.pdf -o ./out -p 2,4 -s vertical
```

ファイル名のプレフィックスを変更:

```bash
pdfsplit input.pdf -o ./out --prefix chapter1
# → chapter1_001.pdf, chapter1_002.pdf, ...
```

## 依存クレート

- [`clap`](https://crates.io/crates/clap) — CLI引数パーサー
- [`lopdf`](https://crates.io/crates/lopdf) — PDF読み書き
- [`anyhow`](https://crates.io/crates/anyhow) — エラーハンドリング

## ライセンス

未設定。
