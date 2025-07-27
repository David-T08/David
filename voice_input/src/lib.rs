use cpal::{
    traits::{DeviceTrait, HostTrait},
    Device, Host,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("No usable input device found")]
    NoInputDevice,
    #[error("Invalid device name")]
    InvalidDeviceName,
    #[error("CPAL error: {0}")]
    Cpal(#[from] cpal::DevicesError),
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
            return Some(d.name().unwrap())
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
            .filter_map(|device| {
                device.name().ok().filter(|name| !is_virtual_input(name))
            })
            .collect();
    
        Ok(names)
    }

    pub fn set_input(&mut self, chosen: Option<String>) -> Result<String, VoiceError> {
        let devices: Vec<Device> = self
            .host
            .input_devices()?
            .filter(|d| {
                d.name()
                    .map(|n| !is_virtual_input(&n))
                    .unwrap_or(false)
            })
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
        let fallback = devices.into_iter().next().ok_or(VoiceError::NoInputDevice)?;
        self.input = Some(fallback.clone());
    
        fallback.name().map_err(|_| VoiceError::InvalidDeviceName)
    }
}
