/// Streaming receiver with VIS detection and bounded buffering.
#[derive(Debug, Default)]
pub struct MartinM1Receiver {
    buffer: Vec<f32>,
    image_start: Option<usize>,
    search_from: usize,
    last_vis: Option<u8>,
    frequency_offset_hz: Option<f32>,
    tuning_offset_hz: f32,
}

impl MartinM1Receiver {
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.image_start = None;
        self.search_from = 0;
        self.last_vis = None;
        self.frequency_offset_hz = None;
    }

    pub fn progress(&self) -> Option<f32> {
        let start = self.image_start?;
        Some(
            ((self.buffer.len().saturating_sub(start)) as f64 / ms_samples(IMAGE_MS) as f64)
                .clamp(0.0, 1.0) as f32,
        )
    }

    /// Most recent parity-valid VIS code observed since reset.
    pub fn detected_vis(&self) -> Option<u8> {
        self.last_vis
    }

    /// Frequency correction inferred from the 1900 Hz VIS leader.
    pub fn frequency_offset_hz(&self) -> Option<f32> {
        self.frequency_offset_hz
    }

    /// Move the expected SSTV tone plan within the captured audio channel.
    pub fn set_tuning_offset_hz(&mut self, offset_hz: f32) {
        let offset_hz = offset_hz.clamp(-1_000.0, 1_000.0);
        if (self.tuning_offset_hz - offset_hz).abs() >= 1.0 {
            self.reset();
            self.tuning_offset_hz = offset_hz;
        }
    }

    pub fn push(&mut self, samples: &[f32]) -> Option<DecodedImage> {
        self.buffer.extend_from_slice(samples);
        if self.image_start.is_none() {
            self.find_header();
        }
        if let Some(start) = self.image_start {
            let needed = start + ms_samples(IMAGE_MS);
            if self.buffer.len() >= needed {
                let decoded = decode_image_audio(
                    &self.buffer[start..needed],
                    self.frequency_offset_hz.unwrap_or(0.0),
                );
                self.reset();
                return decoded.ok();
            }
        } else {
            let keep = ms_samples(HEADER_MS + 400.0);
            if self.buffer.len() > keep {
                let drain = self.buffer.len() - keep;
                self.buffer.drain(..drain);
                self.search_from = self.search_from.saturating_sub(drain);
            }
        }
        None
    }

    fn find_header(&mut self) {
        let header = ms_samples(HEADER_MS);
        let step = ms_samples(10.0);
        while self.search_from + header <= self.buffer.len() {
            if let Some((vis, frequency_offset_hz)) = decode_vis_header(
                &self.buffer[self.search_from..self.search_from + header],
                self.tuning_offset_hz,
            ) {
                self.last_vis = Some(vis);
                self.frequency_offset_hz = Some(frequency_offset_hz);
                if vis == VIS_CODE_MARTIN_M1 {
                    self.image_start = Some(self.search_from + header);
                    return;
                }
                // Skip this complete unsupported header while retaining enough
                // trailing audio to find the next transmission.
                self.search_from += header;
                continue;
            }
            self.search_from += step;
        }
    }
}
