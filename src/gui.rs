use eframe::egui;
use crate::hc12_decoder::{HC12Config, HC12Decoder, DecodeResult};
use crate::rtlsdr::RTLSDRController;
use crate::visualizer::SignalVisualizer;

pub struct HC12DecoderApp {
    config: HC12Config,
    decoder: HC12Decoder,
    rtlsdr: Option<RTLSDRController>,
    visualizer: SignalVisualizer,
    last_result: Option<DecodeResult>,
    running: bool,
    frame_count: usize,
}

impl Default for HC12DecoderApp {
    fn default() -> Self {
        let config = HC12Config::default();
        Self {
            decoder: HC12Decoder::new(config),
            config,
            rtlsdr: RTLSDRController::new().ok(),
            visualizer: SignalVisualizer::new(),
            last_result: None,
            running: false,
            frame_count: 0,
        }
    }
}

impl eframe::App for HC12DecoderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process samples
        if self.running {
            if let Some(rtlsdr) = &self.rtlsdr {
                if let Some(samples) = rtlsdr.get_samples() {
                    let result = self.decoder.decode(&samples);
                    self.last_result = Some(result);
                    self.frame_count += 1;
                }
            }
            ctx.request_repaint();
        }

        // Top panel - Controls
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.heading("🛰 HC12 RTL-SDR Decoder");
            
            ui.separator();
            
            // Status indicator
            ui.horizontal(|ui| {
                let device_running = self.rtlsdr.as_ref()
                    .map(|r| r.is_device_running())
                    .unwrap_or(false);
                
                let status_color = if device_running {
                    egui::Color32::GREEN
                } else {
                    egui::Color32::from_rgb(255, 165, 0) // Orange
                };
                
                ui.colored_label(status_color, "●");
                ui.label(if device_running { 
                    "RTL-SDR Connected" 
                } else { 
                    "Simulation Mode" 
                });
                
                ui.separator();
                ui.label(format!("Frames: {}", self.frame_count));
            });
            
            ui.separator();

            egui::Grid::new("controls_grid")
                .num_columns(2)
                .spacing([20.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Frequency (MHz):");
                    let mut freq_mhz = self.config.frequency / 1_000_000.0;
                    if ui.add(
                        egui::DragValue::new(&mut freq_mhz)
                            .speed(0.1)
                            .range(50.0..=2000.0)
                            .suffix(" MHz")
                    ).changed() {
                        self.config.frequency = freq_mhz * 1_000_000.0;
                        if let Some(rtlsdr) = &self.rtlsdr {
                            rtlsdr.set_frequency(self.config.frequency as u32);
                        }
                    }
                    ui.end_row();

                    ui.label("Gain (dB):");
                    let mut gain = self.config.gain as f32 / 10.0;
                    if ui.add(
                        egui::DragValue::new(&mut gain)
                            .speed(0.1)
                            .range(0.0..=40.0)
                            .suffix(" dB")
                    ).changed() {
                        self.config.gain = (gain * 10.0 as i32;
                        if let Some(rtlsdr) = &self.rtlsdr {
                            rtlsdr.set_tuner_gain(self.config.gain);
                        }
                    }
                    ui.end_row();
                    ui.label("Bandwidth:");
                    let mut bw_khz = self.config.bandwidth / 1000.0;
                    if ui.add(
                        egui::DragValue::new(&mut bw_khz)
                            .speed(1.0)
                            .range(7.8..=500.0)
                            .suffix(" kHz")
                    ).changed() {
                        self.config.bandwidth = bw_khz * 1000.0;
                        self.decoder.update_config(self.config);
                    }
                    ui.end_row();

                    ui.label("Spreading Factor:");
                    if ui.add(
                        egui::Slider::new(&mut self.config.spreading_factor, 7..=12)
                            .text("SF")
                    ).changed() {
                        self.decoder.update_config(self.config);
                    }
                    ui.end_row();

                    ui.label("Code Rate:");
                    if ui.add(
                        egui::Slider::new(&mut self.config.code_rate, 5..=8)
                            .text("4/")
                    ).changed() {
                        self.decoder.update_config(self.config);
                    }
                    ui.end_row();
                });

            ui.separator();

            ui.horizontal(|ui| {
                let button_text = if self.running { "⏸ Pause" } else { "▶ Start" };
                if ui.button(button_text).clicked() {
                    self.running = !self.running;
                }
                
                if ui.button("🔄 Reset").clicked() {
                    self.last_result = None;
                    self.frame_count = 0;
                }
            });
        });

        // Main area
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(result) = &self.last_result {
                    ui.heading("📊 Signal Processing Pipeline");
                    ui.add_space(10.0);
                    
                    // Stage 1
                    ui.group(|ui| {
                        egui::CollapsingHeader::new("1️⃣ Raw IQ Constellation")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("Visualizes the raw I/Q samples from the SDR");
                                self.visualizer.plot_constellation(ui, &result.raw_samples);
                            });
                    });

                    ui.add_space(8.0);

                    // Stage 2
                    ui.group(|ui| {
                        egui::CollapsingHeader::new("2️⃣ Dechirped Signal Spectrum")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("Signal after dechirping with reference chirp");
                                self.visualizer.plot_spectrum(ui, &result.dechirped);
                            });
                    });

                    ui.add_space(8.0);

                    // Stage 3
                    ui.group(|ui| {
                        egui::CollapsingHeader::new("3️⃣ Extracted Symbols")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("Symbols extracted via FFT peak detection");
                                self.visualizer.plot_symbols(ui, &result.symbols);
                                
                                let preview_len = result.symbols.len().min(20);
                                ui.horizontal(|ui| {
                                    ui.label("First symbols:");
                                    ui.code(format!("{:?}", &result.symbols[..preview_len]));
                                });
                            });
                    });

                    ui.add_space(15.0);
                    ui.separator();
                    ui.heading("📤 Decoded Output");
                    ui.add_space(10.0);
                    
                    // Hex output
                    ui.group(|ui| {
                        ui.strong("🔢 Hexadecimal:");
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut result.to_hex().as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                            );
                        });
                    });

                    ui.add_space(8.0);

                    // Binary output
                    ui.group(|ui| {
                        ui.strong("💻 Binary:");
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut result.to_binary().as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                            );
                        });
                    });

                    ui.add_space(8.0);

                    // ASCII/UTF-8 output
                    ui.group(|ui| {
                        ui.strong("📝 ASCII/UTF-8:");
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut result.to_ascii().as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                            );
                        });
                    });
                    
                    ui.add_space(8.0);
                    
                    // Byte count
                    ui.horizontal(|ui| {
                        ui.label(format!("📦 Decoded: {} bytes", result.decoded_bytes.len()));
                    });
                    
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(100.0);
                        ui.heading("⏸ No Data");
                        ui.label("Click 'Start' to begin decoding");
                    });
                }
            });
        });
    }
}
