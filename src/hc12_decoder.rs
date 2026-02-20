use num_complex::Complex32;
use rustfft::{FftPlanner, num_complex::Complex};

pub struct HC12Decoder {
    spreading_factor: u8,
    bandwidth: u32,
    fft_size: usize,
    fft_planner: FftPlanner<f32>,
}

pub struct DecodeResult {
    pub symbols: Vec<u16>,
    pub bytes: Vec<u8>,
    pub snr: f32,
}

impl HC12Decoder {
    pub fn new(spreading_factor: u8, bandwidth: u32) -> Self {
        let fft_size = 1 << spreading_factor; // 2^SF
        
        Self {
            spreading_factor,
            bandwidth,
            fft_size,
            fft_planner: FftPlanner::new(),
        }
    }
    
    pub fn decode(&mut self, samples: &[Complex32]) -> Result<DecodeResult, String> {
        if samples.is_empty() {
            return Err("No samples provided".to_string());
        }
        
        // Detect preamble and synchronize
        let sync_offset = self.detect_preamble(samples);
        
        // Dechirp and extract symbols
        let symbols = self.extract_symbols(samples, sync_offset);
        
        // Convert symbols to bytes (with proper SF handling)
        let bytes = self.symbols_to_bytes(&symbols);
        
        // Calculate SNR estimate
        let snr = self.estimate_snr(samples);
        
        Ok(DecodeResult {
            symbols,
            bytes,
            snr,
        })
    }
    
    fn detect_preamble(&self, samples: &[Complex32]) -> usize {
        // Simplified preamble detection
        // Real implementation would correlate against known preamble chirp
        
        let window_size = self.fft_size;
        let mut max_power = 0.0f32;
        let mut best_offset = 0;
        
        for offset in (0..samples.len().saturating_sub(window_size)).step_by(window_size / 4) {
            let window = &samples[offset..offset + window_size.min(samples.len() - offset)];
            let power: f32 = window.iter().map(|c| c.norm_sqr()).sum();
            
            if power > max_power {
                max_power = power;
                best_offset = offset;
            }
        }
        
        best_offset
    }
    
    fn extract_symbols(&mut self, samples: &[Complex32], offset: usize) -> Vec<u16> {
        let mut symbols = Vec::new();
        let sf = self.spreading_factor as usize;
        let symbol_size = 1 << sf;
        
        // Generate base downchirp for dechirping
        let downchirp = self.generate_downchirp();
        
        let mut pos = offset;
        while pos + symbol_size <= samples.len() {
            // Extract one symbol worth of samples
            let symbol_samples: Vec<Complex32> = samples[pos..pos + symbol_size]
                .iter()
                .cloned()
                .collect();
            
            // Dechirp by multiplying with downchirp
            let dechirped: Vec<Complex<f32>> = symbol_samples.iter()
                .zip(downchirp.iter())
                .map(|(s, d)| {
                    let product = s * d;
                    Complex::new(product.re, product.im)
                })
                .collect();
            
            // FFT to find peak frequency (symbol value)
            let symbol = self.fft_peak_detect(&dechirped);
            symbols.push(symbol);
            
            pos += symbol_size;
        }
        
        symbols
    }
    
    fn generate_downchirp(&self) -> Vec<Complex32> {
        use std::f32::consts::PI;
        
        let n = 1 << self.spreading_factor;
        let mut chirp = Vec::with_capacity(n);
        
        for i in 0..n {
            let t = i as f32 / n as f32;
            // Downchirp: frequency decreases from +BW/2 to -BW/2
            let phase = 2.0 * PI * (0.5 * t - 0.5 * t * t);
            chirp.push(Complex32::new(phase.cos(), -phase.sin())); // Conjugate for downchirp
        }
        
        chirp
    }
    
    fn fft_peak_detect(&mut self, samples: &[Complex<f32>]) -> u16 {
        let mut buffer: Vec<Complex<f32>> = samples.to_vec();
        
        // Pad to FFT size if needed
        buffer.resize(self.fft_size, Complex::new(0.0, 0.0));
        
        // Perform FFT
        let fft = self.fft_planner.plan_fft_forward(self.fft_size);
        fft.process(&mut buffer);
        
        // Find peak bin
        let mut max_magnitude = 0.0f32;
        let mut peak_bin = 0usize;
        
        for (i, sample) in buffer.iter().enumerate() {
            let magnitude = sample.norm_sqr();
            if magnitude > max_magnitude {
                max_magnitude = magnitude;
                peak_bin = i;
            }
        }
        
        // Symbol value is the peak bin, wrapped to SF bits
        let mask = (1u16 << self.spreading_factor) - 1;
        (peak_bin as u16) & mask
    }
    
    fn symbols_to_bytes(&self, symbols: &[u16]) -> Vec<u8> {
        let sf = self.spreading_factor;
        
        match sf {
            // SF < 8: Need to pack multiple symbols into bytes
            7 => self.pack_symbols_to_bytes(symbols, 7),
            
            // SF = 8: Direct 1:1 mapping
            8 => symbols.iter().map(|&s| (s & 0xFF) as u8).collect(),
            
            // SF > 8: Extract most significant byte from each symbol
            9..=12 => {
                symbols.iter()
                    .map(|&s| ((s >> (sf - 8)) & 0xFF) as u8)
                    .collect()
            }
            
            _ => {
                eprintln!("Unsupported spreading factor: {}", sf);
                Vec::new()
            }
        }
    }
    
    /// Pack symbols with fewer than 8 bits into bytes
    fn pack_symbols_to_bytes(&self, symbols: &[u16], sf: u8) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut bit_buffer = 0u32;
        let mut bit_count = 0u8;
        let mask = (1u16 << sf) - 1;
        
        for &symbol in symbols {
            // Add symbol bits to buffer
            bit_buffer = (bit_buffer << sf) | ((symbol & mask) as u32);
            bit_count += sf;
            
            // Extract complete bytes
            while bit_count >= 8 {
                bit_count -= 8;
                let byte = ((bit_buffer >> bit_count) & 0xFF) as u8;
                bytes.push(byte);
            }
        }
        
        // Handle remaining bits (pad with zeros)
        if bit_count > 0 {
            let byte = ((bit_buffer << (8 - bit_count)) & 0xFF) as u8;
            bytes.push(byte);
        }
        
        bytes
    }
    
    fn estimate_snr(&self, samples: &[Complex32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        
        // Simple SNR estimation based on signal variance
        let mean_power: f32 = samples.iter().map(|c| c.norm_sqr()).sum::<f32>() / samples.len() as f32;
        let variance: f32 = samples.iter()
            .map(|c| (c.norm_sqr() - mean_power).powi(2))
            .sum::<f32>() / samples.len() as f32;
        
        if variance > 0.0 {
            10.0 * (mean_power / variance.sqrt()).log10()
        } else {
            0.0
        }
    }
    
    pub fn set_spreading_factor(&mut self, sf: u8) {
        self.spreading_factor = sf.clamp(7, 12);
        self.fft_size = 1 << self.spreading_factor;
    }
    
    pub fn set_bandwidth(&mut self, bw: u32) {
        self.bandwidth = bw;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbols_to_bytes_sf7() {
        let decoder = HC12Decoder::new(7, 125_000);
        
        // 8 symbols of 7 bits = 56 bits = 7 bytes
        let symbols = vec![0x7F, 0x00, 0x55, 0x2A, 0x7F, 0x00, 0x55, 0x2A];
        let bytes = decoder.symbols_to_bytes(&symbols);
        
        assert_eq!(bytes.len(), 7);
    }
    
    #[test]
    fn test_symbols_to_bytes_sf8() {
        let decoder = HC12Decoder::new(8, 125_000);
        
        let symbols = vec![0x41, 0x42, 0x43]; // "ABC"
        let bytes = decoder.symbols_to_bytes(&symbols);
        
        assert_eq!(bytes, vec![0x41, 0x42, 0x43]);
    }
    
    #[test]
    fn test_symbols_to_bytes_sf12() {
        let decoder = HC12Decoder::new(12, 125_000);
        
        // SF12: 12-bit symbols, extract top 8 bits
        let symbols = vec![0x410, 0x420, 0x430]; // Upper nibble should be extracted
        let bytes = decoder.symbols_to_bytes(&symbols);
        
        assert_eq!(bytes, vec![0x41, 0x42, 0x43]);
    }
}
