use egui_plot::{Line, Plot, PlotPoints, Points};
use num_complex::Complex32;
use egui;
use crate::constants::*;

pub struct SignalVisualizer {
    history_size: usize,
    sample_rate: u32,
    center_frequency: u32,
}

impl SignalVisualizer {
    pub fn new() -> Self {
        Self {
            history_size: 2048,
            sample_rate:  2_048_000, // TODO: get sample rate from main.
            center_frequency: SDR_CENTER_FREQUENCY, // TODO: get center frequency from main.
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

    pub fn plot_magnitude(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        let step = samples.len().max(1) / self.history_size.min(samples.len()).max(1);
        
        Plot::new("magnitude")
            .width(700.0)
            .height(250.0)
            .include_y(0.0)
            .include_y(1.0)
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

    pub fn plot_fft(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        use rustfft::{FftPlanner, num_complex::Complex};

        if samples.len() < 64 {
            ui.label("Not enough samples for FFT");
            return;
        }

        // Compute FFT
        let fft_size = self.history_size.min(samples.len().next_power_of_two());

        // Calculate frequency resolution
        let delta_f = self.sample_rate as f64/ fft_size as f64;

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
            .height(300.0)
            .include_y(-50.0)
            .include_y(60.0)
            .label_formatter(|_name, value| {
                format!("Frequency: {:.1} MHz\nPower: {:.1} dB", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                // Convert to dB scale, show only positive frequencies
                let fft_points: PlotPoints = buffer.iter()
                    .take(fft_size)
                    .enumerate()
                    .map(|(i, c)| {
                        let power_db:  f64 = (10.0 * (c.norm_sqr() + 1e-10).log10()).into();
                        let frequency: f64 = (self.center_frequency as f64 - (fft_size as f64)*delta_f/2.0   + i as f64 * delta_f) / 1_000_000.0;
                        [frequency, power_db]
                    })
                    .collect();

                plot_ui.line(
                    Line::new("FFT", fft_points)
                        .color(egui::Color32::from_rgb(200, 100, 255))
                        .width(1.0)
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
}

impl Default for SignalVisualizer {
    fn default() -> Self {
        Self::new()
    }
}
