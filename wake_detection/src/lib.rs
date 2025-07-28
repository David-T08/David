use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use vosk::Model;

pub struct Detector {
    model: Model,
    wake_word: String,
}

impl Detector {
    pub fn new(model_path: &str, wake_word: &str) -> Option<Self> {
        let model = vosk::Model::new(model_path)?;

        Some(Self {
            model,
            wake_word: wake_word.to_lowercase(),
        })
    }

    pub fn spawn(self, mut audio_rx: mpsc::Receiver<Vec<i16>>) -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel(1);

        let grammar = ["hey david"];
        let mut recognizer =
            vosk::Recognizer::new_with_grammar(&self.model, 16000.0, &grammar).unwrap();

        recognizer.set_partial_words(true);
        recognizer.set_words(true);

        let wake_word = self.wake_word.clone();

        tokio::spawn(async move {
            let mut last_match_time = Instant::now();
            let mut last_hey: Option<f32> = None;

            while let Some(samples) = audio_rx.recv().await {
                recognizer.accept_waveform(&samples).unwrap();

                let full = recognizer.final_result().single().unwrap();
                if full.text.len() <= 0 {
                    continue;
                }

                if full.text.contains(&wake_word) {
                    let _ = tx.send(()).await;

                    last_hey = None;
                    last_match_time = Instant::now();

                    recognizer.reset();
                    continue;
                }

                if let Some(curr) = full.result.get(0) {
                    println!("{} @ {}:{} ({}%)", curr.word, curr.start, curr.end, curr.conf * 100.0);

                    if let Some(prev) = &last_hey {
                        let time_gap = curr.start - prev;
                        let is_confident = curr.conf >= 0.75;
                        let is_waking = curr.word.eq_ignore_ascii_case("david");

                        println!("  {time_gap} {is_confident} {is_waking}");

                        if time_gap < 0.15 && is_confident && is_waking {
                            let _ = tx.send(()).await;

                            last_hey = None;
                            last_match_time = Instant::now();

                            recognizer.reset();
                            continue;
                        }
                    }

                    // Set last hey only if its above 70% accuracy
                    if curr.word.eq_ignore_ascii_case("hey") && curr.conf >= 0.70 {
                        last_hey = Some(curr.end);
                    }
                }

                // Auto reset for safety, maybe not needed?
                if last_match_time.elapsed() > Duration::from_secs(10) {
                    last_match_time = Instant::now();
                    last_hey = None;
                    recognizer.reset();
                }
            }
        });

        rx
    }
}
