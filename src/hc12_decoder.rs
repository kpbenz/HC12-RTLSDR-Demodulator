

use num_complex::Complex32;

pub struct HC12Decoder {
    center_frequency: f32,
    sample_rate: f32,
    freq_deviation: f32,      // Expected frequency deviation (Hz)
    symbol_rate: f32,          // Symbol rate (baud)
    samples_per_symbol: usize,
    pub instant_freq: Vec<f32>,     // Instantaneous frequency samples
    pub filtered_freq: Vec<f32>,     // Filtered, instantaneous frequency samples
}



impl HC12Decoder {
    pub fn new(center_frequency: f32, sample_rate: f32, symbol_rate: f32, freq_deviation: f32) -> Self {
        Self {
            center_frequency,
            sample_rate,
            freq_deviation,
            symbol_rate,
            samples_per_symbol: (sample_rate / symbol_rate) as usize,
            instant_freq: Vec::new(),
            filtered_freq: Vec::new(),
        }
    }

    pub fn demodulate(&mut self, iq_samples: &[Complex32]) -> Result<Vec<f32>, String> {

        if iq_samples.is_empty() {
            return Err("No samples provided".to_string());
        }

        // Stage 1: Extract instantaneous frequency
       self.instant_freq =self.compute_instantaneous_frequency(iq_samples);

        // Stage 2: Low-pass filter to remove noise
        self.filtered_freq = self.lowpass_filter(&self.instant_freq);

        // Stage 3: Symbol timing recovery & decision
        let symbols = self.recover_symbols(&self.filtered_freq);

        Ok(symbols)
    }
    fn compute_instantaneous_frequency(&self, iq: &[Complex32]) -> Vec<f32> {
        let num_samples = iq.len();
        let mut mixed_signal = Vec::with_capacity(num_samples);

        // Move base band to 0Hz
        for n in 0..num_samples {
            let time = n as f32 / self.sample_rate;
            let mixer = Complex32::from_polar(1.0, -2.0 * std::f32::consts::PI * self.center_frequency * time);
            mixed_signal.push(iq[n] * mixer);
        }
        let mut freq = Vec::with_capacity(mixed_signal.len() - 1);

        // Calculate instantaneous frequency and calculate the arithmetic mean.
        let mut mean = 0.0_f64;
        for window in mixed_signal.windows(2) {
            // Phase difference = angle between consecutive samples
            let phase_diff = (window[1] * window[0].conj()).arg();

            // Convert to frequency: Δφ * sample_rate / (2π)
            let instant_freq = phase_diff * self.sample_rate / (2.0 * std::f32::consts::PI);
            mean += instant_freq as f64;

            freq.push(instant_freq);
        }
        mean /= freq.len() as f64;

        // Subtract the mean from each frequency value to remove DC offset
        for value in freq.iter_mut() {
            *value -= mean as f32;
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

