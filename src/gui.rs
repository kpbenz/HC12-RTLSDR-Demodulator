use eframe::egui;
use crate::hc12_decoder::{HC12Config, HC12Decoder, DecodeResult};
use crate::rtlsdr::RTLSDRController;
use crate::visualizer::SignalVisualizer;

pub struct HC12DecoderApp {
    config: HC12Config,
    decoder: HC12Decoder,
    last_result: Option<DecodeResult>,
    frame_count: usize,
}

impl Default for HC12DecoderApp {
    fn default() -> Self {
        let config = HC12Config::default();
        Self {
            decoder: HC12Decoder::new(config),
            config,
            last_result: None,
            running: false,
            frame_count: 0,
        }
    }
}


