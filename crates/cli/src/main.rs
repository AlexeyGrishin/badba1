//! Command-line tool for inspecting badba1 database images.
//!
//! Usage: `cli <image.png|image.bmp>` prints the fields and links detected in
//! the image. The format follows the file extension. The tool is a client of
//! the `backend` library: it loads the database with [`backend::load_db`] and
//! shows each field with the text its type gives.

use std::process::ExitCode;

use backend::{Layout, Rect};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        eprintln!("usage: cli <image.png|image.bmp>");
        eprintln!("Shows the fields and links detected in a badba1 PNG or BMP image.");
        return ExitCode::from(2);
    };
    match backend::load_db(path) {
        Ok(db) => {
            print!("{}", render(path, db.layout()));
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

/// Formats a layout as the text the tool prints.
fn render(path: &str, layout: &Layout) -> String {
    let mut out = format!(
        "{path}: {}x{}, {} fields, {} links, {} records\n",
        layout.width,
        layout.height,
        layout.fields.len(),
        layout.links.len(),
        layout.records.len()
    );
    out.push_str("Fields:\n");
    for f in &layout.fields {
        let b = f.borders;
        out.push_str(&format!(
            "  F{}  outer {}  border t{} r{} b{} l{}  data {}  {}\n",
            f.id,
            rect(f.outer),
            b.top,
            b.right,
            b.bottom,
            b.left,
            rect(f.inner),
            f.cli_text()
        ));
    }
    out.push_str("Links:\n");
    for l in &layout.links {
        let ends: Vec<String> = l.fields.iter().map(|id| format!("F{id}")).collect();
        out.push_str(&format!("  L{}  {}\n", l.id, ends.join(" - ")));
    }
    out.push_str("Records:\n");
    for r in &layout.records {
        let fields: Vec<String> = r.fields.iter().map(|id| format!("F{id}")).collect();
        out.push_str(&format!("  R{}  {}\n", r.id, fields.join(", ")));
    }
    out
}

/// `(left,top)-(right,bottom) WxH`
fn rect(r: Rect) -> String {
    format!(
        "({},{})-({},{}) {}x{}",
        r.left,
        r.top,
        r.right,
        r.bottom,
        r.width(),
        r.height()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use backend::{Borders, Field, Record, Value, types};

    #[test]
    fn no_links_prints_no_link_lines() {
        let layout = Layout {
            width: 10,
            height: 10,
            fields: vec![Field {
                id: 1,
                outer: Rect::new(4, 4, 6, 6),
                inner: Rect::new(5, 5, 5, 5),
                borders: Borders {
                    top: 1,
                    right: 1,
                    bottom: 1,
                    left: 1,
                },
                kind: types::TYPES[0],
                color: None,
                value: Value::Bitmap {
                    width: 1,
                    height: 1,
                },
            }],
            links: vec![],
            records: vec![Record {
                id: 1,
                fields: vec![1],
            }],
        };
        let text = render("x.png", &layout);
        assert!(text.starts_with("x.png: 10x10, 1 fields, 0 links, 1 records\n"));
        assert!(text.contains("Links:\nRecords:\n"));
        assert!(text.ends_with("Records:\n  R1  F1\n"));
        assert!(text.contains("data (5,5)-(5,5) 1x1  image\n"));
    }
}
