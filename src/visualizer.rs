use egui_plot::{Line, Plot, PlotPoints, Points};
use rustfft::{FftPlanner, num_complex::Complex32};
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
            history_size: 4096,
            sample_rate:  SDR_SAMPLE_RATE, // TODO: get sample rate from main.
            center_frequency: SDR_CENTER_FREQUENCY, // TODO: get center frequency from main.
        }
    }

    pub fn plot_constellation(&self, ui: &mut egui::Ui, samples: &[Complex32]) {
        let step = samples.len().max(1) / self.history_size.min(samples.len()).max(1);
        
        Plot::new("constellation")
            .view_aspect(1.0)
            .width(250.0)
            .height(250.0)
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
            .include_y(-30.0)
            .label_formatter(|_name, value| {
                format!("Sample: {:.0}\nMagnitude: {:.3} dB", value.x, value.y)
            })
            .show(ui, |plot_ui| {
                let magnitude: PlotPoints = samples.iter()
                    .step_by(step.max(1))
                    .enumerate()
                    .take(self.history_size)
                    .map(|(i, c)| [i as f64, (10.0 * (c.norm() + 1e-10).log10()) as f64])
                    .collect();
                
                plot_ui.line(
                    Line::new("Magnitude", magnitude)
                        .color(egui::Color32::from_rgb(255, 200, 100))
                        .width(1.5)
                );
            });
    }

    fn compute_shifted_spectrum(&self, iq_samples: &[Complex32]) -> (Vec<f32>, Vec<f32>) {
        let n = iq_samples.len();
        let mut buffer = iq_samples.to_vec();

        // Plan and execute FFT
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n);
        fft.process(&mut buffer);

        // Compute magnitude and apply fftshift
        let half = n / 2;
        let mut shifted_mags = vec![0.0; n];

        // FFT shift: swap halves
        for i in 0..half {
            shifted_mags[i] = 10.0 * (buffer[i + half].norm() + 10e-12).log10();
            shifted_mags[i + half] =  10.0 * (buffer[i].norm() + 10e-12).log10();
        }

        // Generate frequency axis (shifted)
        let freq_axis: Vec<f32> = (0..n)
        .map(|k| {
            let offset = (k as f32 - half as f32) * self.sample_rate as f32 / n as f32;
            (self.center_frequency as f32 + offset) / 1_000_000.0
        })
        .collect();

        (freq_axis, shifted_mags)
    }


    pub fn plot_fft(&self, ui: &mut egui::Ui, samples: &[Complex32]) {

        if samples.len() < 64 {
            ui.label("Not enough samples for FFT");
            return;
        }

        let mut buffer: Vec<Complex32> = samples.to_vec();

        let (freqs, mags) = self.compute_shifted_spectrum(&buffer);

        let fft_points: Vec<[f64; 2]> = freqs.iter().zip(mags.iter())
            .map(|(&f, &m)| [f as f64, m as f64])
            .collect();

        Plot::new("fft")
            .width(970.0)
            .height(250.0)
            .include_y(-30.0)
            .include_y(50.0)
            .label_formatter(|_name, value| {
                format!("Frequency: {:.3} MHz\nPower: {:.1} dB", value.x, value.y)
            })
            .show(ui, |plot_ui| {
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
