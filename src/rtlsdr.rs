use crossbeam_channel::{Sender, Receiver, unbounded};
use num_complex::Complex32;
use std::thread;
use std::sync::{Arc, Mutex};

pub struct RTLSDRController {
    sample_rx: Receiver<Vec<Complex32>>,
    control_tx: Option<Sender<RTLSDRCommand>>,
    is_running: Arc<Mutex<bool>>,
}

pub enum RTLSDRCommand {
    SetFrequency(u32),
    SetSampleRate(u32),
    Stop,
}

impl RTLSDRController {
    pub fn new() -> Result<Self, String> {
        let (sample_tx, sample_rx) = unbounded();
        let (control_tx, control_rx) = unbounded();
        let is_running = Arc::new(Mutex::new(false));
        let is_running_clone = is_running.clone();
        
        thread::spawn(move || {
            Self::rtlsdr_thread(sample_tx, control_rx, is_running_clone);
        });
        
        Ok(Self {
            sample_rx,
            control_tx: Some(control_tx),
            is_running,
        })
    }

    ///
    ///
    /// # Arguments
    ///
    /// * `sample_tx`:
    /// * `control_rx`:
    /// * `is_running`:
    ///
    /// returns: ()
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    fn rtlsdr_thread(
        sample_tx: Sender<Vec<Complex32>>,
        control_rx: Receiver<RTLSDRCommand>,
        is_running: Arc<Mutex<bool>>,
    ) {
        // Try to initialize RTL-SDR device
        let device_result = rtlsdr::open(0);
        
        let mut device = match device_result {
            Ok(dev) => dev,
            Err(e) => {
                eprintln!("Failed to open RTL-SDR device: {:?}", e);
                eprintln!("Running in simulation mode...");
                *is_running.lock().unwrap() = false;
                
                // Simulation mode - generate test data
                loop {
                    if let Ok(cmd) = control_rx.try_recv() {
                        if matches!(cmd, RTLSDRCommand::Stop) {
                            break;
                        }
                    }
                    
                    // Generate simulated HC12a-like signal
                    let samples = Self::generate_test_signal();
                    sample_tx.send(samples).ok();
                    thread::sleep(std::time::Duration::from_millis(100));
                }
                return;
            }
        };

        // Configure device
        if let Err(e) = device.set_sample_rate(2_048_000) {
            eprintln!("Failed to set sample rate: {:?}", e);
        }
        
        if let Err(e) = device.set_center_freq(915_000_000) {
            eprintln!("Failed to set frequency: {:?}", e);
        }
        
        if let Err(e) = device.set_tuner_gain_mode(false) {
            eprintln!("Failed to set gain mode: {:?}", e);
        }
        
        if let Err(e) = device.reset_buffer() {
            eprintln!("Failed to reset buffer: {:?}", e);
        }

        *is_running.lock().unwrap() = true;

        loop {
            // Check for commands
            if let Ok(cmd) = control_rx.try_recv() {
                match cmd {
                    RTLSDRCommand::SetFrequency(freq) => {
                        device.set_center_freq(freq).ok();
                    }
                    RTLSDRCommand::SetSampleRate(rate) => {
                        device.set_sample_rate(rate).ok();
                    }
                    RTLSDRCommand::Stop => {
                        *is_running.lock().unwrap() = false;
                        break;
                    }
                }
            }

            // Read samples - read_sync takes length and returns Vec<u8>
            match device.read_sync(262144) {
                Ok(buffer) => {
                    let samples = Self::convert_iq(&buffer);
                    sample_tx.send(samples).ok();
                }
                Err(e) => {
                    eprintln!("Read error: {:?}", e);
                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }
        }
    }

    fn convert_iq(buffer: &[u8]) -> Vec<Complex32> {
        buffer.chunks_exact(2)
            .map(|chunk| {
                let i = (chunk[0] as f32 - 127.5) / 127.5;
                let q = (chunk[1] as f32 - 127.5) / 127.5;
                Complex32::new(i, q)
            })
            .collect()
    }

    fn generate_test_signal() -> Vec<Complex32> {
        use std::f32::consts::PI;
        
        let mut samples = Vec::with_capacity(4096);
        let mut rng_state = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u32;
        
        for i in 0..4096 {
            // Simple PRNG
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            let phase = (rng_state as f32 / u32::MAX as f32) * 2.0 * PI;
            
            // Add chirp-like modulation
            let chirp = (i as f32 * 0.001).sin();
            let amp = 0.3 + chirp * 0.2;
            
            samples.push(Complex32::new(
                amp * phase.cos(),
                amp * phase.sin(),
            ));
        }
        
        samples
    }

    pub fn get_samples(&self) -> Option<Vec<Complex32>> {
        self.sample_rx.try_recv().ok()
    }

    pub fn set_frequency(&self, freq: u32) {
        if let Some(tx) = &self.control_tx {
            tx.send(RTLSDRCommand::SetFrequency(freq)).ok();
        }
    }

    pub fn set_sample_rate(&self, rate: u32) {
        if let Some(tx) = &self.control_tx {
            tx.send(RTLSDRCommand::SetSampleRate(rate)).ok();
        }
    }

    pub fn is_device_running(&self) -> bool {
        *self.is_running.lock().unwrap()
    }
}

impl Drop for RTLSDRController {
    fn drop(&mut self) {
        if let Some(tx) = &self.control_tx {
            tx.send(RTLSDRCommand::Stop).ok();
        }
    }
}
