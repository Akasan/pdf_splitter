use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, ValueEnum};
use lopdf::{Document, Object, ObjectId};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "pdfsplit",
    about = "指定したPDFファイルのページを分割して保存するCLI",
    version
)]
struct Cli {
    /// 入力PDFファイル
    input: PathBuf,

    /// 出力先ディレクトリ (存在しない場合は作成)
    #[arg(short, long)]
    output_dir: PathBuf,

    /// 対象ページ指定 ("all" | "3-7" | "1,3,5" | 混在可 "1,3-5,8")
    #[arg(short, long, default_value = "all")]
    pages: String,

    /// ページ内容の分割モード
    #[arg(short, long, value_enum, default_value_t = SplitMode::None)]
    split: SplitMode,

    /// 出力ファイル名のプレフィックス
    #[arg(long, default_value = "page")]
    prefix: String,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum SplitMode {
    /// 分割せずそのまま保存
    None,
    /// 左右に分割 (left / right)
    Horizontal,
    /// 上下に分割 (top / bottom)
    Vertical,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if !cli.input.exists() {
        bail!("入力ファイルが見つかりません: {}", cli.input.display());
    }
    std::fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("出力ディレクトリの作成に失敗: {}", cli.output_dir.display()))?;

    let src = Document::load(&cli.input)
        .with_context(|| format!("PDF読み込み失敗: {}", cli.input.display()))?;

    let total = src.get_pages().len() as u32;
    if total == 0 {
        bail!("PDFにページがありません");
    }

    let targets = parse_pages(&cli.pages, total)?;
    if targets.is_empty() {
        bail!("対象ページが0件です");
    }

    let width = digit_width(total);

    for &page_num in &targets {
        match cli.split {
            SplitMode::None => {
                let out = cli
                    .output_dir
                    .join(format!("{}_{:0w$}.pdf", cli.prefix, page_num, w = width));
                write_page(&src, page_num, None, &out)?;
                println!("✔ {}", out.display());
            }
            SplitMode::Horizontal => {
                for half in [Half::Left, Half::Right] {
                    let out = cli.output_dir.join(format!(
                        "{}_{:0w$}_{}.pdf",
                        cli.prefix,
                        page_num,
                        half.suffix(),
                        w = width
                    ));
                    write_page(&src, page_num, Some(half), &out)?;
                    println!("✔ {}", out.display());
                }
            }
            SplitMode::Vertical => {
                for half in [Half::Top, Half::Bottom] {
                    let out = cli.output_dir.join(format!(
                        "{}_{:0w$}_{}.pdf",
                        cli.prefix,
                        page_num,
                        half.suffix(),
                        w = width
                    ));
                    write_page(&src, page_num, Some(half), &out)?;
                    println!("✔ {}", out.display());
                }
            }
        }
    }

    Ok(())
}

#[derive(Copy, Clone)]
enum Half {
    Left,
    Right,
    Top,
    Bottom,
}

impl Half {
    fn suffix(self) -> &'static str {
        match self {
            Half::Left => "left",
            Half::Right => "right",
            Half::Top => "top",
            Half::Bottom => "bottom",
        }
    }
}

fn digit_width(n: u32) -> usize {
    std::cmp::max(3, n.to_string().len())
}

fn parse_pages(spec: &str, total: u32) -> Result<Vec<u32>> {
    let spec = spec.trim();
    if spec.eq_ignore_ascii_case("all") {
        return Ok((1..=total).collect());
    }

    let mut out = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            let a: u32 = a.trim().parse().with_context(|| format!("数値ではありません: {a}"))?;
            let b: u32 = b.trim().parse().with_context(|| format!("数値ではありません: {b}"))?;
            if a == 0 || b == 0 || a > b {
                bail!("不正な範囲指定: {part}");
            }
            if b > total {
                bail!("範囲がページ数を超えています ({b} > {total})");
            }
            out.extend(a..=b);
        } else {
            let n: u32 = part.parse().with_context(|| format!("数値ではありません: {part}"))?;
            if n == 0 || n > total {
                bail!("ページ番号が範囲外です: {n} (総ページ数: {total})");
            }
            out.push(n);
        }
    }

    out.sort_unstable();
    out.dedup();
    Ok(out)
}

fn write_page(src: &Document, page_num: u32, half: Option<Half>, out: &Path) -> Result<()> {
    let mut doc = src.clone();

    let all: Vec<u32> = doc.get_pages().keys().copied().collect();
    let to_delete: Vec<u32> = all.into_iter().filter(|p| *p != page_num).collect();
    doc.delete_pages(&to_delete);

    if let Some(h) = half {
        let page_id = *doc
            .get_pages()
            .values()
            .next()
            .ok_or_else(|| anyhow!("対象ページが取得できませんでした (page {page_num})"))?;
        let original = resolve_media_box(&doc, page_id)?;
        let new_box = compute_half(original, h);
        set_page_box(&mut doc, page_id, new_box)?;
    }

    doc.prune_objects();
    doc.compress();
    doc.save(out)
        .with_context(|| format!("保存失敗: {}", out.display()))?;
    Ok(())
}

fn resolve_media_box(doc: &Document, page_id: ObjectId) -> Result<[f64; 4]> {
    let mut current = page_id;
    loop {
        let dict = doc.get_object(current)?.as_dict()?;
        if let Ok(mb) = dict.get(b"MediaBox") {
            return parse_rect(mb);
        }
        match dict.get(b"Parent") {
            Ok(parent) => current = parent.as_reference()?,
            Err(_) => bail!("MediaBoxが見つかりません"),
        }
    }
}

fn parse_rect(obj: &Object) -> Result<[f64; 4]> {
    let arr = obj.as_array()?;
    if arr.len() != 4 {
        bail!("MediaBoxの要素数が不正です");
    }
    let mut out = [0f64; 4];
    for (i, v) in arr.iter().enumerate() {
        out[i] = match v {
            Object::Integer(n) => *n as f64,
            Object::Real(r) => *r as f64,
            _ => bail!("MediaBoxの値が数値ではありません"),
        };
    }
    Ok(out)
}

fn compute_half([llx, lly, urx, ury]: [f64; 4], half: Half) -> [f64; 4] {
    let mx = (llx + urx) / 2.0;
    let my = (lly + ury) / 2.0;
    match half {
        Half::Left => [llx, lly, mx, ury],
        Half::Right => [mx, lly, urx, ury],
        Half::Top => [llx, my, urx, ury],
        Half::Bottom => [llx, lly, urx, my],
    }
}

fn set_page_box(doc: &mut Document, page_id: ObjectId, bbox: [f64; 4]) -> Result<()> {
    let arr = Object::Array(bbox.iter().map(|v| Object::Real(*v as f32)).collect());
    let dict = doc.get_object_mut(page_id)?.as_dict_mut()?;
    dict.set("MediaBox", arr.clone());
    dict.set("CropBox", arr);
    Ok(())
}
