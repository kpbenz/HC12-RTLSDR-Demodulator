

use num_complex::Complex32;

pub struct HC12Decoder {
    sample_rate: f32,
    freq_deviation: f32,      // Expected frequency deviation (Hz)
    symbol_rate: f32,          // Symbol rate (baud)
    samples_per_symbol: usize,
}



impl HC12Decoder {
    pub fn new(sample_rate: f32, symbol_rate: f32, freq_deviation: f32) -> Self {
        Self {
            sample_rate,
            freq_deviation,
            symbol_rate,
            samples_per_symbol: (sample_rate / symbol_rate) as usize,
        }
    }

    pub fn demodulate(&self, iq_samples: &[Complex32]) -> Result<Vec<f32>, String> {

        if iq_samples.is_empty() {
            return Err("No samples provided".to_string());
        }

        // Stage 1: Extract instantaneous frequency
        let instant_freq = self.compute_instantaneous_frequency(iq_samples);

        // Stage 2: Low-pass filter to remove noise
        let filtered_freq = self.lowpass_filter(&instant_freq);

        // Stage 3: Symbol timing recovery & decision
        let symbols = self.recover_symbols(&filtered_freq);

        Ok(symbols)
    }
    fn compute_instantaneous_frequency(&self, iq: &[Complex32]) -> Vec<f32> {
        let mut freq = Vec::with_capacity(iq.len() - 1);

        for window in iq.windows(2) {
            // Phase difference = angle between consecutive samples
            let phase_diff = (window[1] * window[0].conj()).arg();

            // Convert to frequency: Δφ * sample_rate / (2π)
            let instant_freq = phase_diff * self.sample_rate / (2.0 * std::f32::consts::PI);

            freq.push(instant_freq);
        }

        freq
    }

    fn lowpass_filter(&self, freq_samples: &[f32]) -> Vec<f32> {
        // Simple moving average filter
        // Cutoff frequency should be around symbol rate
        let window_size = (self.sample_rate / self.symbol_rate) as usize;
        let mut filtered = Vec::with_capacity(freq_samples.len());

        for i in window_size..freq_samples.len() {
            let mean = freq_samples[i-window_size..i].iter().sum::<f32>() / window_size as f32;
            filtered.push(mean);
        }

        filtered
    }

    fn recover_symbols(&self, filtered_freq: &[f32]) -> Vec<f32> {
        let mut symbols = Vec::new();

        // Sample at symbol rate (take one sample per symbol period)
        for chunk in filtered_freq.chunks(self.samples_per_symbol) {
            if chunk.is_empty() { continue; }

            // Average over the symbol period
            let symbol_value = chunk.iter().sum::<f32>() / chunk.len() as f32;
            symbols.push(symbol_value);
        }

        symbols
    }

    fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
        bits.chunks(8).map(|chunk| {
            chunk.iter().enumerate().fold(0u8, |acc, (i, &b)| {
                acc | (if b { 1 << (7-i) } else { 0 })
            })
        }).collect()
    }
}

