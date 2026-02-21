use egui_plot::{Line, Plot, PlotPoints, Points};
use num_complex::Complex32;
use egui;

pub struct SignalVisualizer {
    history_size: usize,
}

impl SignalVisualizer {
    pub fn new() -> Self {
        Self {
            history_size: 2048,
        }
    }

    pub fn plot_constellation(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        let step = samples.len().max(1) / self.history_size.min(samples.len()).max(1);
        
        Plot::new("constellation")
            .view_aspect(1.0)
            .width(300.0)
            .height(300.0)
            .include_x(-1.1)
            .include_x(1.1)
            .include_y(-1.1)
            .include_y(1.1)
            .label_formatter(|_name, value| {
                format!("I: {:.3}\nQ: {:.3}", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                let points: PlotPoints = samples.iter()
                    .step_by(step.max(1))
                    .take(self.history_size)
                    .map(|c| [c.re as f64, c.im as f64])
                    .collect();
                
                plot_ui.points(
                    Points::new("IQ", points)
                        .radius(2.0)
                        .color(egui::Color32::from_rgb(00, 255, 0))
                );
            });
    }

    pub fn plot_spectrum(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        let step = samples.len().max(1) / self.history_size.min(samples.len()).max(1);
        
        Plot::new("spectrum")
            .width(700.0)
            .height(250.0)
            .include_y(0.0)
            .label_formatter(|_name, value| {
                format!("Sample: {:.0}\nMagnitude: {:.3}", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                let magnitude: PlotPoints = samples.iter()
                    .step_by(step.max(1))
                    .enumerate()
                    .take(self.history_size)
                    .map(|(i, c)| [i as f64, c.norm() as f64])
                    .collect();
                
                plot_ui.line(
                    Line::new("Magnitude", magnitude)
                        .color(egui::Color32::from_rgb(255, 200, 100))
                        .width(1.5)
                );
            });
    }

    pub fn plot_symbols(&self, ui: &mut egui::Ui, symbols: &[u16]) {
        if symbols.is_empty() {
            return;
        }
        
        Plot::new("symbols")
            .width(700.0)
            .height(200.0)
            .include_y(0.0)
            .label_formatter(|_name, value| {
                format!("Symbol #{:.0}\nValue: {:.0}", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                let points_data: Vec<[f64; 2]> = symbols.iter()
                    .enumerate()
                    .map(|(i, &s)| [i as f64, s as f64])
                    .collect();
                
                // Line connecting symbols
                let line_points: PlotPoints = points_data.clone().into();
                plot_ui.line(
                    Line::new("Symbol Line", line_points)
                        .color(egui::Color32::from_rgb(100, 255, 100))
                        .width(1.5)
                );
                
                // Points for each symbol
                let marker_points: PlotPoints = points_data.into();
                plot_ui.points(
                    Points::new("Symbols", marker_points)
                        .radius(4.0)
                        .color(egui::Color32::from_rgb(255, 100, 100))
                );
            });
    }
    
    pub fn plot_fft(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        use rustfft::{FftPlanner, num_complex::Complex};
        
        if samples.len() < 64 {
            ui.label("Not enough samples for FFT");
            return;
        }
        
        // Compute FFT
        let fft_size = 1024.min(samples.len().next_power_of_two());
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        
        let mut buffer: Vec<Complex<f32>> = samples.iter()
            .take(fft_size)
            .map(|c| Complex::new(c.re, c.im))
            .collect();
        buffer.resize(fft_size, Complex::new(0.0, 0.0));
        
        fft.process(&mut buffer);
        
        Plot::new("fft")
            .width(700.0)
            .height(200.0)
            .include_y(0.0)
            .label_formatter(|_name, value| {
                format!("Bin: {:.0}\nPower: {:.1} dB", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                // Convert to dB scale, show only positive frequencies
                let fft_points: PlotPoints = buffer.iter()
                    .take(fft_size / 2)
                    .enumerate()
                    .map(|(i, c)| {
                        let power_db = 10.0 * (c.norm_sqr() + 1e-10).log10();
                        [i as f64, power_db as f64]
                    })
                    .collect();
                
                plot_ui.line(
                    Line::new("FFT", fft_points)
                        .color(egui::Color32::from_rgb(200, 100, 255))
                        .width(1.0)
                );
            });
    }
}

impl Default for SignalVisualizer {
    fn default() -> Self {
        Self::new()
    }
}
