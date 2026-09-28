//! Text recognition in images with the OCR engine built into Windows
//! (`Windows.Media.Ocr`): offline, uses the languages installed on the PC.
//! Runs on a background thread, one image at a time.

use crate::db::Db;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use windows::Graphics::Imaging::{BitmapDecoder, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

pub struct Ocr {
    tx: Sender<i64>,
}

impl Ocr {
    /// Start the worker. `on_done(id)` runs after each recognised image.
    pub fn start(db: Arc<Db>, on_done: impl Fn(i64) + Send + 'static) -> Self {
        let (tx, rx) = channel::<i64>();
        let backlog_db = db.clone();
        let backlog_tx = tx.clone();
        std::thread::Builder::new()
            .name("ocr".into())
            .spawn(move || {
                let engine = match OcrEngine::TryCreateFromUserProfileLanguages() {
                    Ok(e) => e,
                    Err(e) => {
                        log::warn!("OCR unavailable: {e}");
                        return;
                    }
                };
                // Images captured before OCR existed, newest first, at low pace.
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(20));
                    if let Ok(rows) = backlog_db.images_without_ocr(5000) {
                        for (id, _) in rows {
                            if backlog_tx.send(id).is_err() {
                                break;
                            }
                        }
                    }
                });
                for id in rx {
                    if !db.get_settings().map(|s| s.ocr_enabled).unwrap_or(true) {
                        continue;
                    }
                    let Ok((kind, file)) = db.content(id) else {
                        continue;
                    };
                    if kind != "image" {
                        continue;
                    }
                    let text = std::fs::read(db.image_path(&file))
                        .ok()
                        .and_then(|png| recognize(&engine, &png).ok())
                        .unwrap_or_default();
                    // An empty result is stored too, so the image is not retried.
                    if db.set_ocr_text(id, &text).is_ok() && !text.is_empty() {
                        on_done(id);
                    }
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
            })
            .expect("failed to start OCR thread");
        Self { tx }
    }

    pub fn enqueue(&self, id: i64) {
        let _ = self.tx.send(id);
    }
}

fn recognize(engine: &OcrEngine, png: &[u8]) -> windows::core::Result<String> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(png)?;
    writer.StoreAsync()?.join()?;
    writer.FlushAsync()?.join()?;
    writer.DetachStream()?;
    stream.Seek(0)?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
    let mut bitmap = decoder.GetSoftwareBitmapAsync()?.join()?;
    let max = OcrEngine::MaxImageDimension()?;
    if bitmap.PixelWidth()? as u32 > max || bitmap.PixelHeight()? as u32 > max {
        // Too large for the engine: skip rather than recognise a crop.
        return Ok(String::new());
    }
    if bitmap.BitmapPixelFormat()? != BitmapPixelFormat::Bgra8 {
        bitmap = SoftwareBitmap::Convert(&bitmap, BitmapPixelFormat::Bgra8)?;
    }
    let result = engine.RecognizeAsync(&bitmap)?.join()?;
    let mut lines = Vec::new();
    for line in result.Lines()? {
        lines.push(line.Text()?.to_string());
    }
    Ok(lines.join("\n"))
}
