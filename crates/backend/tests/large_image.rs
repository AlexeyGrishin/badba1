//! Analyzes a maximum-size (10000×40000, 400 Mpx) database image.
//!
//! Ignored by default: it needs a few GB of memory and takes seconds in a
//! release build (much longer in debug). To run it:
//!
//! ```text
//! cargo run --release -p backend --example gen_big
//! cargo test --release -p backend --test large_image -- --ignored
//! ```
//!
//! The image is `db/big.png`, or the path in `BADBA1_BIG`. If the file is
//! missing the test prints a message and passes.

use std::path::PathBuf;

#[test]
#[ignore = "needs db/big.png from the gen_big example; run in release"]
fn maximum_size_image_is_analyzed() {
    let path = std::env::var_os("BADBA1_BIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../db/big.png"));
    if !path.is_file() {
        eprintln!(
            "skipping: {} not found; generate it with \
             `cargo run --release -p backend --example gen_big`",
            path.display()
        );
        return;
    }
    let db = backend::load_db(&path).unwrap();
    let layout = db.layout();
    assert_eq!((layout.width, layout.height), (10000, 40000));
    assert_eq!(
        u64::from(layout.width) * u64::from(layout.height),
        backend::MAX_PIXELS
    );
    assert_eq!(layout.fields.len(), 72000);
    assert_eq!(layout.records.len(), 9000);
}
