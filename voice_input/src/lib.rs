use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host, Stream,
};

use rubato::{FftFixedInOut, Resampler};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("No usable input device found")]
    NoInputDevice,

    #[error("Invalid device name")]
    InvalidDeviceName,

    #[error("CPAL device error: {0}")]
    DeviceError(#[from] cpal::DevicesError),

    #[error("CPAL stream config error: {0}")]
    StreamConfigError(#[from] cpal::DefaultStreamConfigError),

    #[error("CPAL build stream error: {0}")]
    BuildStreamError(#[from] cpal::BuildStreamError),

    #[error("CPAL play stream error: {0}")]
    PlayStreamError(#[from] cpal::PlayStreamError),
}

fn is_virtual_input(name: &str) -> bool {
    let lowered = name.to_lowercase();
    lowered.contains("pulse")
        || lowered.contains("pipewire")
        || lowered.contains("jack")
        || lowered.contains("oss")
        || lowered.contains("null")
}

pub struct Recorder {
    host: Host,
    input: Option<Device>,
}

impl Recorder {
    pub fn get_input_name(&self) -> Option<String> {
        if let Some(d) = &self.input {
            return Some(d.name().unwrap());
        }

        None
    }
}

impl Recorder {
    pub fn new() -> Self {
        let host = cpal::default_host();

        Recorder { host, input: None }
    }

    pub fn get_inputs(&self) -> Result<Vec<String>, cpal::DevicesError> {
        let devices = self.host.input_devices()?;

        let names = devices
            .filter_map(|device| device.name().ok().filter(|name| !is_virtual_input(name)))
            .collect();

        Ok(names)
    }

    pub fn set_input(&mut self, chosen: Option<String>) -> Result<String, VoiceError> {
        let devices: Vec<Device> = self
            .host
            .input_devices()?
            .filter(|d| d.name().map(|n| !is_virtual_input(&n)).unwrap_or(false))
            .collect();

        // Try what user chose
        if let Some(custom_name) = chosen {
            if let Some(device) = devices.iter().find(|d| {
                d.name()
                    .map(|name| name.to_lowercase().contains(&custom_name.to_lowercase()))
                    .unwrap_or(false)
            }) {
                self.input = Some(device.clone());
                return device.name().map_err(|_| VoiceError::InvalidDeviceName);
            }
        }

        // Get default with preferred keywords
        let preferred_keywords = ["default", "mic", "intel", "usb"];
        if let Some(device) = devices.iter().find(|d| {
            d.name()
                .map(|name| {
                    let lname = name.to_lowercase();
                    preferred_keywords.iter().any(|kw| lname.contains(kw))
                })
                .unwrap_or(false)
        }) {
            self.input = Some(device.clone());
            return device.name().map_err(|_| VoiceError::InvalidDeviceName);
        }

        // Fallback to first non-virtual device
        let fallback = devices
            .into_iter()
            .next()
            .ok_or(VoiceError::NoInputDevice)?;
        self.input = Some(fallback.clone());

        fallback.name().map_err(|_| VoiceError::InvalidDeviceName)
    }

    pub fn start_input_stream(&mut self) -> Result<(Stream, mpsc::Receiver<Vec<i16>>), VoiceError> {
        let device = self.input.as_ref().ok_or(VoiceError::NoInputDevice)?;
        let config = device.default_input_config()?.into();

        let mut resampler = FftFixedInOut::<f32>::new(44100, 16000, 1024, 1).unwrap();
        let (tx, rx) = mpsc::channel(8);

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                // Downmix stereo to mono
                let mono: Vec<f32> = data.chunks(2).map(|s| (s[0] + s[1]) / 2.0).collect();

                if let Ok(resampled) = resampler.process(&[mono], None) {
                    let i16_samples: Vec<i16> = resampled[0]
                        .iter()
                        .map(|s| (*s * i16::MAX as f32) as i16)
                        .collect();

                    let _ = tx.blocking_send(i16_samples);
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )?;

        Ok((stream, rx))
    }
}
