use anyhow::Result;
use clap::{Parser};
use crossbeam_channel::bounded;
use num_complex::Complex32;
use std::thread;
use std::time::Duration;

mod sdr;
mod demodulator;
mod gui;

#[derive(Parser, Debug)]
#[command(author, version, about = "GFSK Decoder for HC-12 module using RTL-SDR", long_about = None)]
struct Args {
    /// Center frequency in MHz
    #[arg(short, long, default_value = "460.2")]
    frequency: f64,

    /// Bit rate in bps
    #[arg(short, long, default_value = "15000")]
    bitrate: u32,

    /// Sample rate in Hz
    #[arg(short, long, default_value = "2048000")]
    sample_rate: u32,

    /// RTL-SDR device index
    #[arg(short, long, default_value = "0")]
    device: u32,

    /// Manual tuner gain in dB, f not set, AGC is used.
    #[arg(long, allow_negative_numbers = true, default_value = "-1")]
    gain: i32,

    /// Turn on fancy GUI output
    #[arg(long, default_value_t = false)]
    gui: bool,
}



fn main() -> Result<()> {
    let args = Args::parse();

    println!("GFSK Decoder for HC-12 Module");
    println!("================================");
    println!("Center Frequency: {} MHz", args.frequency);
    println!("Bit Rate: {} bps", args.bitrate);
    println!("Sample Rate: {} Hz", args.sample_rate);
    if args.gain < 0 {
        println!("Gain: Automatic");
    } else {
        println!("Gain: {} dB", args.gain);
    }
    println!("GUI: {}", if args.gui {"Enabled"} else {"Disabled"});

    thread::sleep(Duration::from_secs(5));

    // SDR thread pushes IQ blocks to decoder thread.
    let (tx, rx) = bounded::<Vec<Complex32>>(8);

    let sdr_cfg = sdr::SdrConfig {
        device_index: args.device,
        center_frequency: ( args.frequency  * 1_000_000.0 ) as u32,
        sample_rate_hz: args.sample_rate,
        gain_db: args.gain,
    };

    let mut demodulator = demodulator::GfskDemodulator::new(
        args.sample_rate,
        args.bitrate,
        2.0
    );

    std::thread::spawn(move || {
        if let Err(e) = sdr::run_sdr_loop(sdr_cfg, tx) {
            eprintln!("SDR error: {e:#}");
        }
    });


    if args.gui {
        let _ = gui::run(rx);
    } else {

        while let Ok(iq_cmplx32) = rx.recv() {
            let bits = demodulator.process(iq_cmplx32);

            let result: String = bits.iter()
                .map(|&b| if b { '1' } else { '0' })
                .collect();
            // Print the result followed by a newline
            println!("{}", result);

            // flush for interactive output
            use std::io::Write;
            std::io::stdout().flush().ok();
        }

    }
    Ok(())
}
