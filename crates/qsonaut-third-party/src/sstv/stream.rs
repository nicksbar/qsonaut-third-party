#[derive(Debug, Default)]
pub struct MultiModeReceiver {
    buffer: Vec<f32>,
    transmission_start: Option<usize>,
    active_mode: Option<SstvMode>,
    selected_mode: Option<SstvMode>,
    auto_target: bool,
    locked_offset_hz: Option<f32>,
    search_from: usize,
    last_vis: Option<u8>,
    frequency_offset_hz: Option<f32>,
    tuning_offset_hz: f32,
    last_decode_error: Option<String>,
    last_completed_mode: Option<SstvMode>,
    scan_candidate_offset_hz: Option<f32>,
    scan_prominence_db: Option<f32>,
    unresolved_auto_samples: Option<usize>,
    auto_reacquired: bool,
}

impl MultiModeReceiver {
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.transmission_start = None;
        self.active_mode = None;
        self.search_from = 0;
        self.last_vis = None;
        self.frequency_offset_hz = None;
        self.locked_offset_hz = None;
        self.last_decode_error = None;
        self.last_completed_mode = None;
        self.scan_candidate_offset_hz = None;
        self.scan_prominence_db = None;
        self.unresolved_auto_samples = None;
        self.auto_reacquired = false;
    }

    pub fn set_selected_mode(&mut self, mode: Option<SstvMode>) {
        if self.selected_mode != mode {
            self.reset();
            self.selected_mode = mode;
        }
    }

    pub fn set_auto_target(&mut self, enabled: bool) {
        if self.auto_target != enabled {
            self.reset();
            self.auto_target = enabled;
        }
    }

    pub fn auto_target(&self) -> bool {
        self.auto_target
    }

    pub fn locked_offset_hz(&self) -> Option<f32> {
        self.locked_offset_hz
    }

    pub fn scan_candidate_offset_hz(&self) -> Option<f32> {
        self.scan_candidate_offset_hz
    }

    pub fn scan_prominence_db(&self) -> Option<f32> {
        self.scan_prominence_db
    }

    pub fn selected_mode(&self) -> Option<SstvMode> {
        self.selected_mode
    }

    pub fn active_mode(&self) -> Option<SstvMode> {
        self.active_mode
    }

    pub fn progress(&self) -> Option<f32> {
        let start = self.transmission_start?;
        let mode = self.active_mode?;
        Some(
            (self.buffer.len().saturating_sub(start) as f32 / mode_sample_count_12k(mode) as f32)
                .clamp(0.0, 1.0),
        )
    }

    pub fn detected_vis(&self) -> Option<u8> {
        self.last_vis
    }

    pub fn frequency_offset_hz(&self) -> Option<f32> {
        self.frequency_offset_hz
    }

    pub fn take_decode_error(&mut self) -> Option<String> {
        self.last_decode_error.take()
    }

    pub fn take_completed_mode(&mut self) -> Option<SstvMode> {
        self.last_completed_mode.take()
    }

    /// Reports that Auto Target discarded a VIS acquisition which never
    /// resolved to an accepted image mode.
    pub fn take_auto_reacquired(&mut self) -> bool {
        std::mem::take(&mut self.auto_reacquired)
    }

    pub fn set_tuning_offset_hz(&mut self, offset_hz: f32) {
        let offset_hz = offset_hz.clamp(-1_000.0, 1_000.0);
        if (self.tuning_offset_hz - offset_hz).abs() >= 1.0 {
            self.reset();
            self.tuning_offset_hz = offset_hz;
        }
    }

    pub fn push(&mut self, samples: &[f32]) -> Option<DecodedImage> {
        if let Some(elapsed) = &mut self.unresolved_auto_samples {
            *elapsed = elapsed.saturating_add(samples.len());
        }
        self.buffer.extend_from_slice(samples);
        if self.transmission_start.is_none() {
            self.find_header();
        }
        if self
            .unresolved_auto_samples
            .is_some_and(|samples| samples >= ms_samples(AUTO_REACQUIRE_TIMEOUT_MS))
        {
            self.last_vis = None;
            self.frequency_offset_hz = None;
            self.locked_offset_hz = None;
            self.unresolved_auto_samples = None;
            self.auto_reacquired = true;
        }
        if let (Some(start), Some(mode)) = (self.transmission_start, self.active_mode) {
            let needed = start + mode_sample_count_12k(mode) + ms_samples(STREAM_DECODE_GUARD_MS);
            if self.buffer.len() >= needed {
                let result = decode_mode_12k(
                    mode,
                    &self.buffer[start..needed],
                    self.frequency_offset_hz.unwrap_or_default(),
                );
                self.reset();
                return match result {
                    Ok(image) => {
                        self.last_completed_mode = Some(mode);
                        Some(image)
                    }
                    Err(error) => {
                        self.last_decode_error = Some(error.to_string());
                        None
                    }
                };
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
        // Five milliseconds keeps the narrow VIS break/start windows aligned
        // even when a transmission begins between audio callback boundaries.
        let step = ms_samples(5.0);
        while self.search_from + header <= self.buffer.len() {
            let header_audio = &self.buffer[self.search_from..self.search_from + header];
            let detection = if self.auto_target {
                let scan = decode_vis_header_auto(header_audio);
                self.scan_candidate_offset_hz = scan.strongest_offset_hz;
                self.scan_prominence_db = scan.prominence_db;
                scan.detection
            } else {
                decode_vis_header(header_audio, self.tuning_offset_hz)
            };
            if let Some((vis, frequency_offset_hz)) = detection {
                self.last_vis = Some(vis);
                self.frequency_offset_hz = Some(frequency_offset_hz);
                self.locked_offset_hz = Some(frequency_offset_hz);
                if let Some(detected_mode) = mode_from_vis(vis) {
                    if self.selected_mode.is_none() || self.selected_mode == Some(detected_mode) {
                        self.transmission_start = Some(self.search_from);
                        self.active_mode = Some(detected_mode);
                        self.unresolved_auto_samples = None;
                        return;
                    }
                }
                if self.auto_target && self.unresolved_auto_samples.is_none() {
                    self.unresolved_auto_samples = Some(0);
                }
                self.search_from += header;
                continue;
            }
            self.search_from += step;
        }
    }
}
