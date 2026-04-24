use eframe::egui;
use mioffice_pdf_utils::extract_pages;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct App {
    input_path: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    page_count: Option<usize>,

    start_page: usize,
    end_page: usize,
    split_at: usize,

    progress: Arc<Mutex<f32>>,
    working: bool,

    status: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input_path: None,
            page_count: None,
            start_page: 1,
            end_page: 1,
            split_at: 1,
            status: String::new(),
            progress: Arc::new(Mutex::new(0.0)),
            working: false,
            output_dir: None,
        }
    }
}

impl App {
    fn load_pdf(&mut self, path: PathBuf) {
        match lopdf::Document::load(&path) {
            Ok(doc) => {
                self.page_count = Some(doc.get_pages().len());
                self.input_path = Some(path);
                self.status = "PDF loaded".into();
            }
            Err(e) => {
                self.status = format!("Error loading PDF: {e}");
            }
        }
    }

    fn extract_range(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.input_path.as_ref().ok_or("No file selected")?;

        if self.start_page == 0 || self.end_page == 0 {
            return Err("Pages must be >= 1".into());
        }
        if self.start_page > self.end_page {
            return Err("Invalid range".into());
        }

        let input = std::fs::read(path)?;

        let pages: Vec<usize> =
            ((self.start_page - 1)..=(self.end_page - 1)).collect();

        let output = extract_pages(&input, &pages)?;

        let out_path = path.with_file_name("output_range.pdf");
        std::fs::write(out_path, output)?;

        Ok(())
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 🟢 Drag & drop support
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            if let Some(file) = &ctx.input(|i| i.raw.dropped_files[0].clone()).path {
                self.load_pdf(file.clone());
            }
        }

        if self.working {
            let progress = *self.progress.lock().unwrap();

            if progress >= 1.0 {
                self.working = false;
                self.status = "Done!".into();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("PDF Splitter");

            // File picker
            if ui.button("Choose PDF").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PDF", &["pdf"])
                    .pick_file()
                {
                    self.load_pdf(path);
                }
            }

            if ui.button("Choose Output Folder").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    self.output_dir = Some(dir);
                }
            }

            if let Some(dir) = &self.output_dir {
                ui.label(format!("Output: {}", dir.display()));
            }

            if let Some(path) = &self.input_path {
                ui.label(format!("File: {}", path.display()));
            }

            if let Some(count) = self.page_count {
                ui.label(format!("Pages: {}", count));
            }

            ui.separator();

            // 📄 Extract range
            ui.heading("Extract Range");

            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.start_page).range(1..=9999));
                ui.label("to");
                ui.add(egui::DragValue::new(&mut self.end_page).range(1..=9999));
            });

            if ui.button("Extract").clicked() {
                match self.extract_range() {
                    Ok(_) => self.status = "Range extracted!".into(),
                    Err(e) => self.status = format!("Error: {e}"),
                }
            }

            ui.separator();

            // ✂️ Split into chunks
            ui.heading("Split into chunks");

            ui.horizontal(|ui| {
                ui.label("Pages per file:");
                ui.add(egui::DragValue::new(&mut self.split_at).range(1..=9999));
            });

            if ui.add_enabled(!self.working, egui::Button::new("Split")).clicked() && !self.working {
                *self.progress.lock().unwrap() = 0.0;
                let path = self.input_path.clone();
                let out_dir = self.output_dir.clone();
                let total = self.page_count;
                let split_at = self.split_at;

                let progress = self.progress.clone();

                self.working = true;
                self.status = "Processing...".into();

                std::thread::spawn(move || {
                    if let (Some(path), Some(out_dir), Some(total)) = (path, out_dir, total) {

                        let input = std::fs::read(&path).unwrap();

                        let mut start = 0;
                        let mut index = 1;

                        while start < total {
                            let end = (start + split_at).min(total);

                            let pages: Vec<usize> = (start..end).collect();
                            let output = extract_pages(&input, &pages).unwrap();

                            let out_name = format!("part_{index}.pdf");
                            let out_path = out_dir.join(out_name);

                            std::fs::write(out_path, output).unwrap();

                            start = end;
                            index += 1;

                            // update progress
                            let mut prog = progress.lock().unwrap();
                            *prog = start as f32 / total as f32;
                        }
                    }
                });
            }


            ui.separator();
            
            let progress = *self.progress.lock().unwrap();

            ui.add(
                egui::ProgressBar::new(progress)
                    .show_percentage()
            );

            ui.label(&self.status);
        });
    }
    
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        
    }
}

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "PDF Splitter",
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}