
/// Default sample rate for RTLSDR dongle.
pub const SDR_SAMPLE_RATE: u32 = 280_000;

/// Default center frequency
pub const SDR_DEFAULT_CENTER_FREQUENCY: u32 = 460_200_000;

/// Default gain setting fot the RTLSDR dongle
pub const SDR_DEFAULT_GAIN: i32 = 300;

/// Default  buffersize for IQ asynchronous read
pub const SDR_BUFFER_SIZE: usize = 0x20000;