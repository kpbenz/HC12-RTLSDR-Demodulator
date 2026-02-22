
mod constants;
mod rtlsdr;
mod hc12_decoder;
mod visualizer;

use constants::SDR_SAMPLE_RATE;
use eframe::egui;
use egui::load::Result;
use num_complex::Complex32;
use rtlsdr::RTLSDRController;
use hc12_decoder::HC12Decoder;
use visualizer::SignalVisualizer;

#[derive(Debug, Clone, Copy, PartialEq)]
enum BitRate {
    Rate5000,
    Rate15000,
    Rate58000,
    Rate236000,
}

impl BitRate {
    fn as_value(self) -> u32 {
        match self {
            BitRate::Rate5000 => 5000,
            BitRate::Rate15000 => 15000,
            BitRate::Rate58000 => 58000,
            BitRate::Rate236000 => 236000,
        }
    }
    fn as_string(self) -> String {
        match self {
            BitRate::Rate5000 => "5000".to_string(),
            BitRate::Rate15000 => "15000".to_string(),
            BitRate::Rate58000 => "58000".to_string(),
            BitRate::Rate236000 => "236000".to_string(),
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("HC12 RTLSDR Demodulator"),
        ..Default::default()
    };
    
    eframe::run_native(
        "HC12 RTL-SDR Demodulator",
        options,
        Box::new(|_cc| Ok(Box::new(HC12App::new()))),
    )
}

struct HC12App {
    rtlsdr: Option<RTLSDRController>,
    decoder: HC12Decoder,
    visualizer: SignalVisualizer,
    
    // Settings
    frequency: u32,
    gain: i32,
    bit_rate: BitRate,
    sample_rate: u32,
    spreading_factor: u8,
    bandwidth: u32,

    // State
    current_samples: Vec<Complex32>,
    decoded_symbols: Vec<u16>,
    decoded_bytes: Vec<u8>,
    decoded_text: String,
    status_message: String,
    is_running: bool,
}

impl HC12App {
    fn new() -> Self {
        let rtlsdr = match RTLSDRController::new() {
            Ok(controller) => {
                println!("RTL-SDR initialized successfully");
                Some(controller)
            }
            Err(e) => {
                eprintln!("Failed to initialize RTL-SDR: {}", e);
                eprintln!("Running in simulation mode");
                None
            }
        };
        
        Self {
            rtlsdr,
            decoder: HC12Decoder::new(7, 125_000),
            visualizer: SignalVisualizer::new(),
            
            frequency: constants::SDR_CENTER_FREQUENCY,
            gain: 300,
            bit_rate: BitRate::Rate15000,
            sample_rate: SDR_SAMPLE_RATE,
            spreading_factor: 7,
            bandwidth: 125_000,
            
            current_samples: Vec::new(),
            decoded_symbols: Vec::new(),
            decoded_bytes: Vec::new(),
            decoded_text: String::new(),
            status_message: String::from("Ready"),
            is_running: false,
        }
    }
    
    fn process_samples(&mut self) {
        if let Some(ref rtlsdr) = self.rtlsdr {
            if let Some(samples) = rtlsdr.get_samples() {
                self.current_samples = samples.clone();
                
                // Decode HC12 signal
                match self.decoder.decode(&samples) {
                    Ok(result) => {
                        self.decoded_symbols = result.symbols.clone();
                        self.decoded_bytes = result.bytes.clone();
                        
                        // Try to convert to text
                        if let Ok(text) = String::from_utf8(result.bytes.clone()) {
                            if !text.is_empty() && text.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                                self.decoded_text = text;
                            }
                        }
                        
                        self.status_message = format!(
                            "Decoded {} symbols, {} bytes",
                            result.symbols.len(),
                            result.bytes.len()
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Decode error: {}", e);
                    }
                }
            }
        } else {
            // Simulation mode - generate test data
            self.current_samples = Self::generate_test_samples();
            if let Ok(result) = self.decoder.decode(&self.current_samples) {
                self.decoded_symbols = result.symbols;
                self.decoded_bytes = result.bytes;
            }
        }
    }
    
    fn generate_test_samples() -> Vec<Complex32> {
        use std::f32::consts::PI;

        let mut samples = Vec::with_capacity(4096);
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as f32 / 1000.0;

        for i in 0..4096 {
            let phase = 2.0 * PI * (i as f32 / 4096.0 + t * 0.1);
            let chirp = (i as f32 * 0.001 + t).sin() * 0.5;
            let noise = (i as f32 * 12345.6789).sin() * 0.1;

            samples.push(Complex32::new(
                (phase + chirp).cos() * 0.5 + noise,
                (phase + chirp).sin() * 0.5 + noise,
            ));
        }

        samples
    }
}

impl eframe::App for HC12App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process samples if running
        if self.is_running {
            self.process_samples();
            ctx.request_repaint();
        }
        
        // Top panel - Controls
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("HC12 RTL-SDR Decoder");
                ui.separator();
                
                if ui.button(if self.is_running { "⏹ Stop" } else { "▶ Start" }).clicked() {
                    self.is_running = !self.is_running;
                }
                
                ui.separator();
                ui.label(&self.status_message);
            });
        });
        
        // Left panel - Settings
        egui::SidePanel::left("settings").min_width(250.0).show(ctx, |ui| {
            ui.heading("Settings");
            ui.separator();
            
            ui.label("Frequency:");
            let mut freq_mhz = self.frequency as f32 / 1_000_000.0;
            if ui.add(egui::Slider::new(&mut freq_mhz, 24.0..=1766.0)
                .fixed_decimals(3)
                .step_by(0.1)
                .suffix(" MHz")).changed() {
                self.frequency = (freq_mhz * 1_000_000.0) as u32;
                if let Some(ref rtlsdr) = self.rtlsdr {
                    rtlsdr.set_frequency(self.frequency);
                }
            }
            
            ui.separator();

            ui.label("Gain:");
            let mut gain_db = self.gain as f32 / 10.0;
            if ui.add(egui::Slider::new(&mut gain_db, 0.0..=40.0)
                .fixed_decimals(1)
                .step_by(0.1)
                .suffix(" dB")).changed() {
                self.gain = (gain_db * 10.0) as i32;
                if let Some(ref rtlsdr) = self.rtlsdr {
                    rtlsdr.set_gain(self.gain);
                }
            }

            ui.separator();

            ui.label("Bitrate:");
            if ui.radio_value(&mut self.bit_rate, BitRate::Rate5000, BitRate::Rate5000.as_string()).clicked() {
                self.bit_rate = BitRate::Rate5000;
            }
            if ui.radio_value(&mut self.bit_rate, BitRate::Rate15000, BitRate::Rate15000.as_string()).clicked() {
                self.bit_rate = BitRate::Rate15000;
            }
            if ui.radio_value(&mut self.bit_rate, BitRate::Rate58000, BitRate::Rate58000.as_string()).clicked() {
                self.bit_rate = BitRate::Rate58000;
            }
            if ui.radio_value(&mut self.bit_rate, BitRate::Rate236000, BitRate::Rate236000.as_string()).clicked() {
                self.bit_rate = BitRate::Rate236000;
            }


            ui.separator();
            
            ui.label("Bandwidth:");
            egui::ComboBox::from_label("")
                .selected_text(format!("{} kHz", self.bandwidth / 1000))
                .show_ui(ui, |ui| {
                    for bw in [125_000u32, 250_000, 500_000] {
                        if ui.selectable_value(&mut self.bandwidth, bw, format!("{} kHz", bw / 1000)).clicked() {
                            self.decoder = HC12Decoder::new(self.spreading_factor, self.bandwidth);
                        }
                    }
                });
            
            ui.separator();
            ui.heading("Statistics");
            
            ui.label(format!("Samples: {}", self.current_samples.len()));
            ui.label(format!("Symbols: {}", self.decoded_symbols.len()));
            ui.label(format!("Bytes: {}", self.decoded_bytes.len()));
            
            if let Some(ref rtlsdr) = self.rtlsdr {
                ui.separator();
                ui.label(if rtlsdr.is_device_running() {
                    "🟢 Device: Connected"
                } else {
                    "🟡 Device: Simulation"
                });
            } else {
                ui.separator();
                ui.label("🔴 Device: Not found");
            }
        });
        
        // Central panel - Visualizations
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Constellation diagram
                ui.heading("IQ Constellation");
                if !self.current_samples.is_empty() {
                    self.visualizer.plot_constellation(ui, &self.current_samples);
                } else {
                    ui.label("No data");
                }
                
                ui.separator();
                
                // Magnitude
                ui.heading("Signal Magnitude");
                if !self.current_samples.is_empty() {
                    self.visualizer.plot_magnitude(ui, &self.current_samples);
                } else {
                    ui.label("No data");
                }

                ui.separator();

                // Spectrum
                ui.heading("Signal Spectrum");
                if !self.current_samples.is_empty() {
                    self.visualizer.plot_fft(ui, &self.current_samples);
                } else {
                    ui.label("No data");
                }

                ui.separator();

                // Decoded symbols
                ui.heading("Decoded Symbols");
                if !self.decoded_symbols.is_empty() {
                    self.visualizer.plot_symbols(ui, &self.decoded_symbols);
                } else {
                    ui.label("No symbols decoded");
                }
                
                ui.separator();
                
                // Decoded data
                ui.heading("Decoded Data");
                ui.horizontal_wrapped(|ui| {
                    ui.label("Hex:");
                    let hex_str: String = self.decoded_bytes.iter()
                        .map(|b| format!("{:02X} ", b))
                        .collect();
                    ui.monospace(&hex_str);
                });
                
                if !self.decoded_text.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Text:");
                        ui.monospace(&self.decoded_text);
                    });
                }
            });
        });
    }
}
