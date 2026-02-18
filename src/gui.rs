use eframe::{egui};
use egui_plot::{Plot, Points};
use num_complex::{Complex32};
use crossbeam_channel::Receiver;


pub struct GuiApp {
    iq_data: Vec<Complex32>, // Vector to hold IQ complex values
    receiver: Option<Receiver<Vec<Complex32>>>,
}


impl Default for GuiApp {
    fn default() -> Self {
        Self {
            iq_data: Vec::new(),
            receiver: None,
        }
    }
}

impl GuiApp {

    // Method to update the IQ data
    pub fn update_iq_data(&mut self, new_data: Vec<Complex32>) {
        self.iq_data = new_data;
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("IQ Data");

            if let Some(receiver) = &self.receiver {
                if let Ok(iq_cmplx32) = receiver.recv() {
                    self.update_iq_data(iq_cmplx32);
                }
            }

            if !self.iq_data.is_empty() {
                // Prepare the data for plotting
                let scatter_data: Vec<[f64;2]> = self
                    .iq_data
                    .iter()
                    .map(|c | [c.re as f64, c.im as f64])
                    .collect();
                // Create the plot
                Plot::new("IQ Plot")
                    .view_aspect(1.0)
                    .show(ui, |plot_ui| {
                        plot_ui.points(
                            Points::new("IQ".to_string(), scatter_data)
                                .color(egui::Color32::from_rgb(0, 255, 0)))
                    });
                // Request a repaint to update the plot
                ctx.request_repaint();
            } else {
                ui.label("No data available to plot.");
            }
        });
    }
}

pub fn run(rx:Receiver<Vec<Complex32>>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("HC12 RTLSDR Demodulator", options, Box::new(|_cc| Ok(Box::new(GuiApp {
        iq_data: vec![],
        receiver: Some(rx),
    }))))
}
