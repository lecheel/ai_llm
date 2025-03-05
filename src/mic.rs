use console::{Style, Term};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn mic_main() -> Result<bool, Box<dyn std::error::Error>> {
    // Initialize CPAL host
    let host = cpal::default_host();

    // Get default input device
    let device = host
        .default_input_device()
        .expect("No input device available");
    //println!("Using input device: {}", device.name()?);

    // Configure audio stream
    let config = device.default_input_config()?;
    //println!("Input config: {:?}", config);

    // Shared state for stopping the stream
    let stop_flag = Arc::new(Mutex::new(false));

    // Setup terminal for capturing keypresses
    let term = Term::stdout();
    let stop_flag_clone = Arc::clone(&stop_flag);
    std::thread::spawn(move || {
        let _ = term.read_char(); // Wait for any key press
        let mut stop = stop_flag_clone.lock().unwrap();
        *stop = true;
    });

    // Start time for timeout
    let start_time = Instant::now();

    // Create a WAV writer and wrap it in an Arc<Mutex<Option<...>>>
    let spec = WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let writer = WavWriter::create("/tmp/output.wav", spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    // Clone the writer for use in the audio callback
    let writer_clone = Arc::clone(&writer);

    // Define the audio callback function
    let err_fn = |err| eprintln!("An error occurred on the input audio stream: {}", err);
    let vu_meter = Arc::new(Mutex::new(0.0_f32));
    let vu_meter_clone = Arc::clone(&vu_meter);

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            // Lock the writer to write audio data to the WAV file
            let mut writer_guard = writer_clone.lock().unwrap();
            if let Some(writer) = writer_guard.as_mut() {
                for &sample in data {
                    writer.write_sample(sample).unwrap();
                }
            }

            // Update VU meter
            let max_sample = data.iter().map(|s| s.abs()).fold(0.0_f32, |a, b| a.max(b));
            let mut vu = vu_meter_clone.lock().unwrap();
            *vu = max_sample;
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    //println!("Recording... Press ENTER to stop or wait for 30 seconds.");

    // Main loop to monitor VU meter and handle stop condition
    loop {
        let vu_level = {
            let vu = vu_meter.lock().unwrap();
            *vu
        };

        // Determine color and label based on VU level
        let (style, label) = if vu_level < 0.3 {
            (Style::new().green(), " Low  ")
        } else if vu_level < 0.7 {
            (Style::new().yellow(), "Medium")
        } else {
            (Style::new().red(), " High ")
        };

        // Display colored VU meter
        let bar = "=".repeat((vu_level * 50.0) as usize);
        print!(
            "\rVU Meter: [{}] {:.2} ({})",
            style.apply_to(format!("{:<50}", bar)),
            vu_level,
            style.apply_to(label)
        );
        io::stdout().lock().flush().unwrap();

        // Check if stop flag is set or timeout reached
        let elapsed = start_time.elapsed();
        let should_stop = {
            let stop = stop_flag.lock().unwrap();
            *stop || elapsed >= Duration::from_secs(30)
        };

        if should_stop {
            break;
        }

        // Sleep briefly to prevent busy-waiting
        std::thread::sleep(Duration::from_millis(100));
    }

    // Stop the stream
    drop(stream);

    // Finalize the WAV file
    {
        let mut writer_guard = writer.lock().unwrap();
        if let Some(writer) = writer_guard.take() {
            writer.finalize()?;
        }
    }
    return Ok(true);
}
