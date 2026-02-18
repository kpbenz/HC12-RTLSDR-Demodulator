use num_complex::{Complex32};
use std::f32::consts::PI;

pub struct GfskDemodulator {
    sample_rate: u32,
    bitrate: u32,
    samples_per_bit: f32,
    
    // Demodulation state
    prev_phase: f32,
    
    // Bit synchronization
    bit_buffer: Vec<f32>,
    bit_position: f32,
    
    // Byte decoding
    byte_buffer: Vec<bool>,
    
    // Statistics and visualization data
    pub fm_demod_output: Vec<f32>,
    pub filtered_output: Vec<f32>,
    pub bit_decisions: Vec<bool>,
    
    // Low-pass filter state
    lpf_state: Vec<f32>,
    lpf_coeffs: Vec<f32>,
    deviation: f32,
}

impl GfskDemodulator {
    pub fn new(sample_rate: u32, bitrate: u32, deviation: f32) -> Self {
        let samples_per_bit = sample_rate as f32 / bitrate as f32;
        
        // Design low-pass filter based on signal bandwidth
        // GFSK bandwidth ≈ 2 * (deviation + bitrate/2)
        // Filter should pass the signal bandwidth
        let signal_bandwidth = 2.0 * (deviation + bitrate as f32 / 2.0);
        let filter_taps = (sample_rate as f32 / signal_bandwidth).max(5.0) as usize;
        let filter_taps = filter_taps.min(64); // Cap at 64 taps
        
        let lpf_coeffs = vec![1.0 / filter_taps as f32; filter_taps];

        Self {
            sample_rate,
            bitrate,
            deviation,
            samples_per_bit,
            prev_phase: 0.0,
            bit_buffer: Vec::new(),
            bit_position: 0.0,
            byte_buffer: Vec::new(),
            fm_demod_output: Vec::new(),
            filtered_output: Vec::new(),
            bit_decisions: Vec::new(),
            lpf_state: vec![0.0; filter_taps],
            lpf_coeffs,
        }
    }

    /// Process IQ samples and return demodulated bits
    pub fn process(&mut self, iq_samples: Vec<Complex32>) -> Vec<bool> {
        // Clear visualization buffers
        self.fm_demod_output.clear();
        self.filtered_output.clear();
        self.bit_decisions.clear();

        // Step 1: FM Demodulation (frequency discriminator)
        let fm_output = self.fm_discriminator(iq_samples);
        self.fm_demod_output = fm_output.clone();

        // Step 2: Low-pass filtering
        let filtered = self.low_pass_filter(&fm_output);
        self.filtered_output = filtered.clone();

        // Step 3: Bit recovery with timing synchronization
        let bits = self.recover_bits(&filtered);
        self.bit_decisions = bits.clone();

        bits
    }

    /// FM discriminator - detects instantaneous frequency and normalizes by deviation
    fn fm_discriminator(&mut self, iq_samples: Vec<Complex32>) -> Vec<f32> {
        let mut output = Vec::with_capacity(iq_samples.len());

        for sample in iq_samples {
            // Calculate phase
            let phase = sample.im.atan2(sample.re);
            
            // Phase difference (unwrapped)
            let mut phase_diff = phase - self.prev_phase;
            
            // Unwrap phase (-π to π)
            while phase_diff > PI {
                phase_diff -= 2.0 * PI;
            }
            while phase_diff < -PI {
                phase_diff += 2.0 * PI;
            }
            
            // Convert phase difference to frequency (Hz)
            let freq = phase_diff * self.sample_rate as f32 / (2.0 * PI);
            
            // Normalize by deviation to get baseband signal
            // For GFSK: bit "1" → +deviation, bit "0" → -deviation
            // After normalization: bit "1" → +1, bit "0" → -1
            let normalized = freq / self.deviation;
            
            output.push(normalized);
            
            self.prev_phase = phase;
        }

        output
    }

    /// Apply low-pass filter to remove high-frequency noise
    fn low_pass_filter(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = Vec::with_capacity(input.len());

        for &sample in input {
            // Shift state buffer
            self.lpf_state.rotate_right(1);
            self.lpf_state[0] = sample;

            // Convolution
            let filtered: f32 = self.lpf_state.iter()
                .zip(self.lpf_coeffs.iter())
                .map(|(s, c)| s * c)
                .sum();

            output.push(filtered);
        }

        output
    }

    /// Recover bits from filtered FM output with symbol timing recovery
    fn recover_bits(&mut self, filtered: &[f32]) -> Vec<bool> {
        let mut bits = Vec::new();

        // Threshold-based bit slicer with timing recovery
        for &sample in filtered {
            self.bit_buffer.push(sample);
            self.bit_position += 1.0;

            // Check if we've accumulated enough samples for one bit
            if self.bit_position >= self.samples_per_bit {
                // Sample at the middle of the bit period
                let mid_index = (self.bit_buffer.len() / 2).min(self.bit_buffer.len() - 1);
                let bit_value = self.bit_buffer[mid_index];

                // Threshold decision
                // After normalization by deviation: positive → 1, negative → 0
                bits.push(bit_value > 0.0);

                // Reset for next bit
                self.bit_buffer.clear();
                self.bit_position -= self.samples_per_bit;
            }
        }

        bits
    }

    /// Decode bits into bytes (LSB first)
    pub fn decode_bytes(&mut self, bits: &[bool]) -> Option<Vec<u8>> {
        // Add bits to buffer
        self.byte_buffer.extend_from_slice(bits);

        let mut bytes = Vec::new();

        // Extract complete bytes (8 bits each)
        while self.byte_buffer.len() >= 8 {
            let mut byte: u8 = 0;
            
            // LSB first encoding
            for i in 0..8 {
                if self.byte_buffer[i] {
                    byte |= 1 << i;
                }
            }

            bytes.push(byte);
            
            // Remove processed bits
            self.byte_buffer.drain(0..8);
        }

        if bytes.is_empty() {
            None
        } else {
            Some(bytes)
        }
    }

    /// Get current statistics for visualization
    pub fn get_stats(&self) -> DemodStats {
        DemodStats {
            samples_per_bit: self.samples_per_bit,
            bit_buffer_size: self.bit_buffer.len(),
            byte_buffer_size: self.byte_buffer.len(),
            deviation: self.deviation,
            bitrate: self.bitrate,
            signal_bandwidth: 2.0 * (self.deviation + self.bitrate as f32 / 2.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DemodStats {
    pub samples_per_bit: f32,
    pub bit_buffer_size: usize,
    pub byte_buffer_size: usize,
    pub deviation: f32,
    pub bitrate: u32,
    pub signal_bandwidth: f32,
}
