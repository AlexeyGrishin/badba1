//! The database handle: the library's single entry point.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};

use crate::detect::detect;
use crate::draw::{plan, read_back, render};
use crate::error::Error;
use crate::formats::{self, Format, Storage};
use crate::layout::{Field, Layout, Record, RecordId};
use crate::types::{Input, Value};

/// Opens the database stored in the image file at `path`.
///
/// The file's format is chosen by its extension (see
/// [`formats::format_for`]). The image is read and analyzed once; the
/// returned [`Database`] answers every later read from memory (and, for
/// formats that keep pixels on disk, from the file's rows) and does not see
/// changes other programs make to the file.
///
/// # Errors
///
/// [`Error::Io`] if the file cannot be read, [`Error::Decode`] if it is not a
/// valid file of its format, [`Error::Unsupported`] for a variant of the
/// format that is not supported, and [`Error::TooLarge`] for an image of more
/// than [`MAX_PIXELS`](crate::MAX_PIXELS) pixels.
#[doc(alias = "open")]
pub fn load_db(path: impl AsRef<Path>) -> Result<Database, Error> {
    Database::load(path)
}

/// A loaded database: the layout detected in its image, the values of its
/// fields, and where its pixels are stored.
///
/// Records are found by their [`RecordId`] and fields by their index in the
/// record (from 0, in record order). Reads take `&self`; [`set_field`]
/// takes `&mut self`, so share a database between threads behind a lock.
///
/// [`set_field`]: Database::set_field
pub struct Database {
    path: PathBuf,
    format: &'static dyn Format,
    storage: Box<dyn Storage>,
    layout: Layout,
    /// Record ids, indexed by record number (index 0 is unused).
    ids: Vec<RecordId>,
    /// Record numbers by id.
    by_id: HashMap<RecordId, usize>,
    background: Rgba<u8>,
}

impl Database {
    /// Opens the database stored in the image file at `path`; the same as
    /// [`load_db`].
    pub fn load(path: impl AsRef<Path>) -> Result<Database, Error> {
        let path = path.as_ref();
        let (format, image) = formats::read_image(path)?;
        let layout = detect(&image);
        let background = *image.get_pixel(0, 0);
        let storage = format.storage(path, image)?;
        let mut ids = vec![RecordId { x: 0, y: 0 }; layout.records.len() + 1];
        let mut by_id = HashMap::with_capacity(layout.records.len());
        for r in &layout.records {
            let first = &layout.fields[r.fields[0] - 1];
            let id = RecordId {
                x: first.outer.left,
                y: first.outer.top,
            };
            ids[r.id] = id;
            by_id.insert(id, r.id);
        }
        Ok(Database {
            path: path.to_path_buf(),
            format,
            storage,
            layout,
            ids,
            by_id,
            background,
        })
    }

    /// The file the database was loaded from, and is saved to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The name of the file's storage format, such as `"png"`.
    pub fn format(&self) -> &'static str {
        self.format.name()
    }

    /// Image width in pixels.
    pub fn width(&self) -> u32 {
        self.layout.width
    }

    /// Image height in pixels.
    pub fn height(&self) -> u32 {
        self.layout.height
    }

    /// The background color: the image's top-left pixel.
    pub fn background(&self) -> Rgba<u8> {
        self.background
    }

    /// Everything detected in the image: fields, links and records, with
    /// their numbers, and the fields' current values.
    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// The records, in record order.
    #[doc(alias = "get_records")]
    pub fn records(&self) -> impl ExactSizeIterator<Item = RecordView<'_>> {
        self.layout.records.iter().map(move |record| RecordView {
            db: self,
            record,
        })
    }

    /// The record with id `id`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchRecord`] if there is none.
    #[doc(alias = "get_record")]
    pub fn record(&self, id: RecordId) -> Result<RecordView<'_>, Error> {
        let number = *self.by_id.get(&id).ok_or(Error::NoSuchRecord(id))?;
        Ok(RecordView {
            db: self,
            record: &self.layout.records[number - 1],
        })
    }

    /// The id of record number `number` (starting at 1), as found in a
    /// [`Value::Record`].
    pub fn record_id(&self, number: usize) -> Option<RecordId> {
        (1..self.ids.len()).contains(&number).then(|| self.ids[number])
    }

    /// Field `index` (from 0, in record order) of record `record`.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchRecord`] or [`Error::NoSuchField`].
    #[doc(alias = "get_field")]
    pub fn field(&self, record: RecordId, index: usize) -> Result<FieldView<'_>, Error> {
        self.record(record)?.field(index)
    }

    /// The pixels of the data area of field `index` of record `record`, as
    /// currently stored.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchRecord`] or [`Error::NoSuchField`]; [`Error::Io`] if a
    /// format that keeps pixels on disk cannot read the file.
    #[doc(alias = "get_field_image")]
    pub fn field_image(&self, record: RecordId, index: usize) -> Result<RgbaImage, Error> {
        let inner = self.field(record, index)?.field().inner;
        self.storage.read_rect(inner)
    }

    /// Writes `input` into field `index` of record `record`, saves the change
    /// to the file in its format, and returns the field with its new value,
    /// read back from the stored pixels.
    ///
    /// Only the field's data area changes. A failed write changes nothing:
    /// not the file, and not the database in memory.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchRecord`] or [`Error::NoSuchField`];
    /// [`Error::Write`] if the field cannot take the value (its type is not
    /// writable, the input is of another kind, or the value does not fit);
    /// [`Error::Save`] if the file cannot be written.
    #[doc(alias = "write_field")]
    pub fn set_field(
        &mut self,
        record: RecordId,
        index: usize,
        input: Input,
    ) -> Result<FieldView<'_>, Error> {
        let id = self.field(record, index)?.field().id;
        let field = &self.layout.fields[id - 1];
        let input = match input {
            Input::Bitmap(mut bitmap) if field.kind.input_kind().is_some() => {
                self.storage.stored(&mut bitmap);
                Input::Bitmap(bitmap)
            }
            other => other,
        };
        let plan = plan(field, input)?;
        let inner = field.inner;
        let area = render(&plan, inner.width(), inner.height(), self.background);
        let value = read_back(field, &area, self.background);
        self.storage.write_rect(inner, &area)?;
        self.layout.fields[id - 1].value = value;
        self.field(record, index)
    }
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .field("format", &self.format.name())
            .field("width", &self.layout.width)
            .field("height", &self.layout.height)
            .field("records", &self.layout.records.len())
            .finish()
    }
}

/// A record of a [`Database`].
#[derive(Clone, Copy)]
pub struct RecordView<'a> {
    db: &'a Database,
    record: &'a Record,
}

impl<'a> RecordView<'a> {
    /// The record's id.
    pub fn id(&self) -> RecordId {
        self.db.ids[self.record.id]
    }

    /// The record's number, starting at 1 (as the CLI shows it, `R1`).
    pub fn number(&self) -> usize {
        self.record.id
    }

    /// The number of fields in the record.
    pub fn len(&self) -> usize {
        self.record.fields.len()
    }

    /// Whether the record has no fields (never true for a detected record).
    pub fn is_empty(&self) -> bool {
        self.record.fields.is_empty()
    }

    /// The record's fields, in record order.
    #[doc(alias = "get_fields")]
    pub fn fields(&self) -> impl ExactSizeIterator<Item = FieldView<'a>> + use<'a> {
        let db = self.db;
        self.record
            .fields
            .iter()
            .enumerate()
            .map(move |(index, &id)| FieldView {
                db,
                field: &db.layout.fields[id - 1],
                index,
            })
    }

    /// Field `index` (from 0, in record order).
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchField`] if the record has fewer fields.
    #[doc(alias = "get_field")]
    pub fn field(&self, index: usize) -> Result<FieldView<'a>, Error> {
        let id = *self.record.fields.get(index).ok_or(Error::NoSuchField {
            record: self.id(),
            index,
        })?;
        Ok(FieldView {
            db: self.db,
            field: &self.db.layout.fields[id - 1],
            index,
        })
    }
}

/// A field of a record of a [`Database`].
#[derive(Clone, Copy)]
pub struct FieldView<'a> {
    db: &'a Database,
    field: &'a Field,
    index: usize,
}

impl<'a> FieldView<'a> {
    /// The field's index in its record, from 0.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The detected field: its number, rectangles, type and value.
    pub fn field(&self) -> &'a Field {
        self.field
    }

    /// The name of the field's type, such as `"counter"`.
    pub fn type_name(&self) -> &'static str {
        self.field.type_name()
    }

    /// The field's value.
    pub fn value(&self) -> &'a Value {
        &self.field.value
    }

    /// For a [`Value::Record`], the id of the referenced record (`None` for
    /// a null reference or another kind of value).
    pub fn target(&self) -> Option<RecordId> {
        match self.field.value {
            Value::Record(Some(n)) => self.db.record_id(n),
            _ => None,
        }
    }

    /// The field's type and value as the CLI shows them, such as `counter 5`.
    pub fn cli_text(&self) -> String {
        self.field.cli_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::WriteError;
    use crate::formats::bmp::encode_bmp;
    use crate::layout::Rect;

    fn testdata(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../testdata")
            .join(name)
    }

    /// A fresh temporary folder, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("badba1-db-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }

        /// A copy of `testdata/<name>.png` in the folder, as PNG or 24-bit BMP.
        fn copy(&self, name: &str, bmp: bool) -> PathBuf {
            if bmp {
                let image = crate::formats::read_image(&testdata(&format!("{name}.png")))
                    .unwrap()
                    .1;
                let path = self.0.join(format!("{name}.bmp"));
                std::fs::write(&path, encode_bmp(&image, 24, false)).unwrap();
                path
            } else {
                let path = self.0.join(format!("{name}.png"));
                std::fs::copy(testdata(&format!("{name}.png")), &path).unwrap();
                path
            }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn id(x: u32, y: u32) -> RecordId {
        RecordId { x, y }
    }

    /// The field whose outer rectangle is `outer`, with its record and index.
    fn find(db: &Database, outer: Rect) -> (RecordId, usize) {
        for r in db.records() {
            for f in r.fields() {
                if f.field().outer == outer {
                    return (r.id(), f.index());
                }
            }
        }
        panic!("no field at {outer:?}");
    }

    #[test]
    fn reference_properties() {
        let db = load_db(testdata("example1.png")).unwrap();
        assert_eq!((db.width(), db.height()), (539, 348));
        assert_eq!(db.format(), "png");
        assert_eq!(db.background(), Rgba([255, 255, 255, 255]));
        let l = db.layout();
        assert_eq!((l.fields.len(), l.links.len(), l.records.len()), (5, 3, 2));
        assert_eq!(db.records().len(), 2);
    }

    #[test]
    fn record_ids_in_order() {
        let db = load_db(testdata("example3.png")).unwrap();
        let ids: Vec<String> = db.records().map(|r| r.id().to_string()).collect();
        assert_eq!(ids, ["35/21", "250/23", "35/68"]);
        assert_eq!(db.record(id(250, 23)).unwrap().number(), 2);
        assert_eq!("250/23".parse::<RecordId>().unwrap(), id(250, 23));
    }

    #[test]
    fn not_found_errors() {
        let db = load_db(testdata("example3.png")).unwrap();
        let err = db.record(id(1, 1)).err().unwrap();
        assert_eq!(err.to_string(), "no record 1/1");
        let err = db.field(id(35, 21), 9).err().unwrap();
        assert!(matches!(err, Error::NoSuchField { index: 9, .. }));
        assert_eq!(err.to_string(), "record 35/21 has no field 9");
    }

    #[test]
    fn values_of_each_kind() {
        let db = load_db(testdata("example1.png")).unwrap();
        let (r, i) = find(&db, Rect::new(101, 22, 137, 47));
        let f = db.field(r, i).unwrap();
        assert_eq!((f.type_name(), f.value()), ("counter", &Value::Int(5)));
        let (r, i) = find(&db, Rect::new(156, 22, 193, 50));
        let f = db.field(r, i).unwrap();
        assert_eq!(f.cli_text(), "float 36.84%");
        assert!(matches!(f.value(), Value::Number(n) if (n - 36.842).abs() < 0.001));
        let (r, i) = find(&db, Rect::new(35, 23, 77, 52));
        let f = db.field(r, i).unwrap();
        assert_eq!(f.type_name(), "image");
        assert!(matches!(f.value(), Value::Bitmap { .. }));

        let db = load_db(testdata("example2.png")).unwrap();
        let (r, i) = find(&db, Rect::new(310, 29, 327, 49));
        assert_eq!(db.field(r, i).unwrap().value(), &Value::Bool(true));
        let (r, i) = find(&db, Rect::new(198, 26, 220, 43));
        let f = db.field(r, i).unwrap();
        let (target, _) = find(&db, Rect::new(250, 23, 280, 52));
        assert_eq!(f.type_name(), "ref");
        assert_eq!(f.target(), Some(target));

        let db = load_db(testdata("example3.png")).unwrap();
        let (r, i) = find(&db, Rect::new(297, 54, 338, 60));
        assert_eq!(
            db.field(r, i).unwrap().value(),
            &Value::Text("OPENSPEC".into())
        );
    }

    #[test]
    fn field_image() {
        let db = load_db(testdata("example3.png")).unwrap();
        let img = db.field_image(id(35, 21), 0).unwrap();
        assert_eq!(img.dimensions(), (33, 20));
        let full = crate::formats::read_image(&testdata("example3.png"))
            .unwrap()
            .1;
        let want = image::imageops::crop_imm(&full, 40, 26, 33, 20).to_image();
        assert_eq!(img, want);
    }

    #[test]
    fn write_and_reload() {
        for bmp in [false, true] {
            let dir = TempDir::new(&format!("write-{bmp}"));
            let path = dir.copy("example3", bmp);
            let mut db = load_db(&path).unwrap();
            let f = db.set_field(id(35, 21), 1, Input::Int(7)).unwrap();
            assert_eq!(f.value(), &Value::Int(7));
            assert_eq!(db.field(id(35, 21), 1).unwrap().value(), &Value::Int(7));
            let again = load_db(&path).unwrap();
            assert_eq!(again.field(id(35, 21), 1).unwrap().value(), &Value::Int(7));
            assert_eq!(again.layout().fields.len(), db.layout().fields.len());
        }
    }

    #[test]
    fn failed_write_changes_nothing() {
        let dir = TempDir::new("overflow");
        let path = dir.copy("example3", false);
        let before = std::fs::read(&path).unwrap();
        let mut db = load_db(&path).unwrap();
        let err = db.set_field(id(35, 21), 1, Input::Int(100_000)).err().unwrap();
        assert!(matches!(err, Error::Write(WriteError::FieldOverflow { .. })));
        assert_eq!(db.field(id(35, 21), 1).unwrap().value(), &Value::Int(4));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn failed_save_rolls_back() {
        let dir = TempDir::new("save");
        let path = dir.copy("example3", false);
        let mut db = load_db(&path).unwrap();
        let image_before = db.field_image(id(35, 21), 0).unwrap();
        std::fs::create_dir(dir.0.join("example3.png.tmp")).unwrap();
        let err = db.set_field(id(35, 21), 1, Input::Int(7)).err().unwrap();
        assert!(matches!(err, Error::Save { .. }), "{err}");
        assert_eq!(db.field(id(35, 21), 1).unwrap().value(), &Value::Int(4));
        let bitmap = RgbaImage::from_pixel(10, 10, Rgba([0, 0, 0, 255]));
        assert!(db.set_field(id(35, 21), 0, Input::Bitmap(bitmap)).is_err());
        assert_eq!(db.field_image(id(35, 21), 0).unwrap(), image_before);
    }

    #[test]
    fn file_replaced_after_loading() {
        let dir = TempDir::new("replaced");
        let path = dir.copy("example3", false);
        let db = load_db(&path).unwrap();
        std::fs::copy(testdata("example1.png"), &path).unwrap();
        let ids: Vec<String> = db.records().map(|r| r.id().to_string()).collect();
        assert_eq!(ids, ["35/21", "250/23", "35/68"]);
    }

    #[test]
    fn bitmap_into_a_bmp_is_stored_opaque() {
        let dir = TempDir::new("opaque");
        let path = dir.copy("example3", true);
        let mut db = load_db(&path).unwrap();
        let bitmap = RgbaImage::from_pixel(10, 10, Rgba([10, 20, 30, 128]));
        db.set_field(id(35, 21), 0, Input::Bitmap(bitmap)).unwrap();
        let img = db.field_image(id(35, 21), 0).unwrap();
        assert_eq!(img.dimensions(), (33, 20));
        assert_eq!(img.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
    }

    #[test]
    fn load_errors() {
        let err = load_db("does-not-exist.png").unwrap_err();
        assert!(matches!(err, Error::Io { .. }));
        assert!(err.to_string().contains("does-not-exist.png"));
    }
}
