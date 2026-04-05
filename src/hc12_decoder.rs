

use std::f32::consts::PI;
use num_complex::Complex32;

pub struct HC12Decoder {
    center_frequency: f32,
    sample_rate: f32,
    freq_deviation: f32,      // Expected frequency deviation (Hz)
    symbol_rate: f32,          // Symbol rate (baud)
    samples_per_symbol: usize,
    pub instant_freq: Vec<f32>,     // Instantaneous frequency samples
    pub filtered_freq: Vec<Complex32>,     // Filtered, instantaneous frequency samples
    filter: Box<LowPassFilter>,
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
            filter: Box::new(LowPassFilter {
                sample_rate: sample_rate,
                cutoff_hz:   freq_deviation,
                num_taps:    259,
            })
        }
    }

    pub fn demodulate(&mut self, iq_samples: &[Complex32]) -> Result<Vec<f32>, String> {

        if iq_samples.is_empty() {
            return Err("No samples provided".to_string());
        }

        // Stage 1: Low-pass filter to remove noise
        self.filtered_freq = self.filter.lowpass_filter(iq_samples);

        // Stage 2: Extract instantaneous frequency
        self.instant_freq =self.compute_instantaneous_frequency(&self.filtered_freq);

        // Stage 3: Symbol timing recovery & decision
        let symbols = self.recover_symbols(&self.instant_freq);

        Ok(symbols)
    }
    fn compute_instantaneous_frequency(&self, iq: &[Complex32]) -> Vec<f32> {
        let num_samples = iq.len();
        let mut freq = Vec::with_capacity(num_samples - 1);

        // Calculate instantaneous frequency and calculate the arithmetic mean.
        let mut mean = 0.0_f64;
        for window in iq.windows(2) {
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

pub struct LowPassFilter {
    pub sample_rate: f32,
    pub cutoff_hz: f32,
    pub num_taps: usize, // filter kernel length (odd recommended)
}

impl LowPassFilter {
    /// Applies a Hamming-windowed sinc low-pass filter to IQ samples.
    /// Input  : time-domain IQ samples (Complex32: real=I, imag=Q)
    /// Output : filtered IQ samples, same length as input
    pub fn lowpass_filter(&self, iq_samples: &[Complex32]) -> Vec<Complex32> {
        let kernel = self.build_kernel();
        let half   = kernel.len() / 2;
        let n      = iq_samples.len();

        (0..n)
            .map(|i| {
                kernel.iter().enumerate().fold(
                    Complex32::new(0.0, 0.0),
                    |acc, (j, &coeff)| {
                        // center the kernel tap index
                        let tap = i + j;
                        if tap < half || tap - half >= n {
                            acc // zero-pad out-of-bounds
                        } else {
                            acc + iq_samples[tap - half] * coeff
                        }
                    },
                )
            })
            .collect()
    }

    /// Builds the normalized Hamming-windowed sinc kernel (real coefficients).
    fn build_kernel(&self) -> Vec<f32> {
        let cutoff_norm = self.cutoff_hz / self.sample_rate; // normalized [0.0, 0.5]
        let m           = self.num_taps;
        let half        = (m / 2) as f32;

        let mut h: Vec<f32> = (0..m)
            .map(|i| {
                let n = i as f32 - half;

                // Sinc component
                let sinc = if n == 0.0 {
                    2.0 * cutoff_norm
                } else {
                    (2.0 * PI * cutoff_norm * n).sin() / (PI * n)
                };

                // Hamming window component
                let hamming = 0.54 - 0.46 * (2.0 * PI * i as f32 / (m - 1) as f32).cos();

                sinc * hamming
            })
            .collect();

        // Normalize → unity DC gain
        let sum: f32 = h.iter().sum();
        h.iter_mut().for_each(|v| *v /= sum);
        h
    }
}

