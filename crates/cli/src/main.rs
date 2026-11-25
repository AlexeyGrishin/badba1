//! Command-line tool for inspecting badba1 database images.
//!
//! Usage: `cli <image.png>` prints the fields and links detected in the image.

use std::path::Path;
use std::process::ExitCode;

use backend::{FieldData, Layout, Rect};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        eprintln!("usage: cli <image.png>");
        eprintln!("Shows the fields and links detected in a badba1 PNG image.");
        return ExitCode::from(2);
    };
    match backend::analyze_png(Path::new(path)) {
        Ok(layout) => {
            print!("{}", render(path, &layout));
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
            data(&f.data)
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

/// The field's type, followed by its value for `float` and `counter` fields.
fn data(d: &FieldData) -> String {
    match d {
        FieldData::Image => "image".to_string(),
        FieldData::Float { .. } => format!("float {:.2}%", d.percent().unwrap_or(0.0)),
        FieldData::Counter { count, .. } => format!("counter {count}"),
        FieldData::Bool { value, .. } => format!("bool {value}"),
    }
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
    use backend::{Borders, Field, Record};

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
                data: FieldData::Image,
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

    #[test]
    fn bool_fields_show_true_and_false() {
        let pink = [255, 174, 201, 255];
        assert_eq!(
            data(&FieldData::Bool {
                color: pink,
                value: true
            }),
            "bool true"
        );
        assert_eq!(
            data(&FieldData::Bool {
                color: pink,
                value: false
            }),
            "bool false"
        );
    }
}
