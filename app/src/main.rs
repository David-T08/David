use cpal::traits::StreamTrait;
use tokio::sync::mpsc;
use voice_input::Recorder;
use wake_detection::Detector;

pub fn spawn_audio_buffer(mut raw_rx: mpsc::Receiver<Vec<i16>>) -> mpsc::Receiver<Vec<i16>> {
    let (tx, out_rx) = mpsc::channel(4);

    tokio::spawn(async move {
        let mut buffer = Vec::new();
        const TARGET_SIZE: usize = 16000 / 3; // ~200ms at 16kHz

        while let Some(chunk) = raw_rx.recv().await {
            buffer.extend_from_slice(&chunk);

            if buffer.len() >= TARGET_SIZE {
                let send_buf = buffer.split_off(0); // swap buffer contents
                let _ = tx.send(send_buf).await;
            }
        }
    });

    out_rx
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut recorder = Recorder::new();
    let input = recorder.set_input(None);

    if let Err(e) = input {
        eprintln!("{}", e);
        return Ok(());
    }

    println!();
    println!("Chose input: {}", recorder.get_input_name().unwrap());
    
    let detector = Detector::new(
        "/home/david/development/rust/david/models/vosk-model-en-us-0.22-lgraph",
        "hey david",
    )
    .unwrap();

    let (stream, raw_audio_rx) = recorder.start_input_stream()?;
    stream.play().unwrap();

    let mut wake_rx = detector.spawn(spawn_audio_buffer(raw_audio_rx));

    while let Some(()) = wake_rx.recv().await {
        println!("Wake word detected!");
    }

    Ok(())
}
