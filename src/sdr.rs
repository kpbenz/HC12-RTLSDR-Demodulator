use anyhow::{anyhow, Context, Result};
use crossbeam_channel::Sender;
use num_complex::{Complex32};
use rtlsdr_mt;

/// User-facing SDR configuration.
#[derive(Debug, Clone)]
pub struct SdrConfig {
    pub device_index: u32,
    pub center_frequency: u32,
    pub sample_rate_hz: u32,
    pub gain_db: i32, // -1 => auto
}

/// Continuously reads raw interleaved IQ bytes (u8) from RTL-SDR and sends blocks to decoder.
///
/// Notes:
/// - `read_async()` calls the callback with a borrowed buffer; we copy into `Vec<u8>`
///   before sending across threads.
/// - If the decoder is slow, the bounded channel will apply backpressure (send will block).
pub fn run_sdr_loop(cfg: SdrConfig, out: Sender<Vec<Complex32>>) -> Result<()> {
    // open() returns a controller (for configuration) and a reader (for streaming)
    let (mut ctrl, mut reader) = rtlsdr_mt::open(cfg.device_index)
        .map_err(|e| anyhow!("Opening RTLSDR device {0:?} failed: {e:?}", cfg.device_index))?;

    // ---- Configure controller ----
    ctrl.set_sample_rate(cfg.sample_rate_hz)
        .map_err(|e| anyhow!("set_sample_rate failed: {e:?}"))?;
    ctrl.set_center_freq(cfg.center_frequency)
        .map_err(|e| anyhow!("set_center_freq failed: {e:?}"))?;

    // Gain: auto or manual (API may vary; adjust here if needed)
    if cfg.gain_db < 0 {
        let _ = ctrl.enable_agc(); // auto
    } else {
        let _ = ctrl.disable_agc();
        let _ = ctrl.set_tuner_gain(cfg.gain_db * 10);
    }

    ctrl.reset_buffer()
        .map_err(|e| anyhow!("reset_buffer failed: {e:?}"))?;


    let buf_num: u32 = 12;          // buf_num: number of USB transfer buffers
    let buf_len: u32 = 32 * 1024;   // buf_len: bytes per buffer

    reader.read_async(buf_num, buf_len, move |data: &[u8]| {
        // Convert interleaved u8 IQ bytes into normalized Complex32 samples
        let iq_samples: Vec<Complex32> = data
            .chunks(2)
            .filter_map(|chunk| {
                if let [i, q] = *chunk {
                    let i_norm = (i as f32 - 127.5) / 127.5; // Normalize to [-1, 1]
                    let q_norm = (q as f32 - 127.5) / 127.5;
                    Some(Complex32::new(i_norm, q_norm))
                } else {
                    None // Drop incomplete trailing chunk, if any
                }
            })
            .collect();

        // Apply backpressure; if you prefer dropping on overload, use try_send
        let _ = out.send(iq_samples);
    })
    .map_err(|e| anyhow!("read_async failed: {e:?}"))
    .context("RTL-SDR async read failed")?;

    Ok(())
}
