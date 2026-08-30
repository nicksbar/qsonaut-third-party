use cwdit_dsp::{Debouncer, Goertzel, MovingAverage, RunLengthEncoder, Threshold};
use cwdit_morse::{BootstrapDecoder, Decoded as UpstreamDecoded, TimingEstimator};

/// Adapter around cw-dit's IO-free DSP/Morse crates.
///
/// One instance represents one selected CW channel. The returned filtered
/// audio is optional channel audio for a consumer-owned monitor or recorder.
pub struct CwChannel {
    audio_filter: CwAudioFilter,
    filter: Goertzel,
    smoother: MovingAverage,
    slicer: ChannelSlicer,
    rle: RunLengthEncoder,
    debouncer: Debouncer,
    decoder: BootstrapDecoder,
    sample_rate_hz: f32,
    block_len: u32,
    text: String,
}

enum ChannelSlicer {
    Classic(Threshold),
}

/// First-party CW events; upstream Morse result types do not cross this API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CwDecode {
    Character(char),
    WordBreak,
    Unknown,
}

/// Streaming 240 Hz-wide audio channel used ahead of the detector.
struct CwAudioFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl CwAudioFilter {
    const BANDWIDTH_HZ: f32 = 240.0;

    fn new(sample_rate_hz: u32, tone_hz: u32) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1) as f32;
        let tone_hz = (tone_hz as f32).clamp(1.0, sample_rate_hz * 0.49);
        let q = (tone_hz / Self::BANDWIDTH_HZ).max(0.5);
        let omega = 2.0 * std::f32::consts::PI * tone_hz / sample_rate_hz;
        let alpha = omega.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: (-2.0 * omega.cos()) / a0,
            a2: (1.0 - alpha) / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn push(&mut self, sample: f32) -> f32 {
        let output = self.b0 * sample + self.z1;
        self.z1 = self.b1 * sample - self.a1 * output + self.z2;
        self.z2 = self.b2 * sample - self.a2 * output;
        output
    }
}

impl CwChannel {
    pub fn new(sample_rate_hz: u32, tone_hz: u32, wpm: u8) -> Self {
        let sample_rate = sample_rate_hz as f32;
        let wpm = f32::from(wpm.clamp(5, 40));
        // Fine enough for period-based mark classification while leaving the
        // Goertzel reasonably selective around a 5–40 WPM audio channel.
        // Match cw-dit's audio decode policy: a quarter-dit integration
        // window preserves keying edges while still averaging RF noise.
        let block_len = ((0.25 * 1.2 / wpm) * sample_rate)
            .round()
            .max((sample_rate / tone_hz as f32).ceil() + 1.0)
            .max(16.0) as u32;
        let envelope_rate = sample_rate / block_len as f32;
        let dit_ticks = 1.2 * envelope_rate / wpm;
        let smoother = MovingAverage::new((dit_ticks / 4.0).round().clamp(2.0, 16.0) as usize);
        let slicer =
            ChannelSlicer::Classic(Threshold::new(envelope_rate, 1.0, 0.005).with_snr_gate(2.5));
        let min_run = (dit_ticks / 5.0).round().max(2.0) as u32;
        let decoder = BootstrapDecoder::new(TimingEstimator::from_wpm(wpm, envelope_rate))
            .with_adapt(true)
            .with_period_classification(true);
        Self {
            audio_filter: CwAudioFilter::new(sample_rate_hz, tone_hz),
            filter: Goertzel::new(tone_hz as f32, sample_rate, block_len),
            smoother,
            slicer,
            rle: RunLengthEncoder::new(),
            debouncer: Debouncer::new(min_run),
            decoder,
            sample_rate_hz: sample_rate,
            block_len,
            text: String::new(),
        }
    }

    pub fn push_samples_with_audio(&mut self, samples: &[f32]) -> (Vec<CwDecode>, Vec<f32>) {
        let mut output = Vec::new();
        let mut channel_audio = Vec::with_capacity(samples.len());
        for &sample in samples {
            let sample = self.audio_filter.push(sample);
            channel_audio.push(sample);
            if let Some(envelope) = self.filter.push(sample) {
                let mark = match &mut self.slicer {
                    ChannelSlicer::Classic(slicer) => slicer.push(self.smoother.push(envelope)),
                };
                if let Some(run) = self.rle.push(mark).and_then(|run| self.debouncer.push(run)) {
                    self.push_run(run.mark, run.duration, &mut output);
                }
            }
        }
        (output, channel_audio)
    }

    /// Flush a completed carrier-off interval so a final Morse character is
    /// emitted even when the stream remains open for later reception.
    pub fn finish(&mut self) -> Vec<CwDecode> {
        let mut output = Vec::new();
        if let Some(run) = self.rle.finish().and_then(|run| self.debouncer.push(run)) {
            self.push_run(run.mark, run.duration, &mut output);
        }
        if let Some(run) = self.debouncer.finish() {
            self.push_run(run.mark, run.duration, &mut output);
        }
        for event in self.decoder.finish() {
            self.accumulate(event);
            output.push(Self::map_event(event));
        }
        output
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn block_len(&self) -> u32 {
        self.block_len
    }

    pub fn envelope_rate(&self) -> f32 {
        self.sample_rate_hz / self.block_len as f32
    }

    fn map_event(event: UpstreamDecoded) -> CwDecode {
        match event {
            UpstreamDecoded::Char(character) => CwDecode::Character(character),
            UpstreamDecoded::WordBreak => CwDecode::WordBreak,
            UpstreamDecoded::Unknown => CwDecode::Unknown,
        }
    }

    fn push_run(&mut self, mark: bool, duration: u32, output: &mut Vec<CwDecode>) {
        for event in self.decoder.push(mark, duration) {
            self.accumulate(event);
            output.push(Self::map_event(event));
        }
    }

    fn accumulate(&mut self, event: UpstreamDecoded) {
        match event {
            UpstreamDecoded::Char(character) => self.text.push(character),
            UpstreamDecoded::WordBreak => self.text.push(' '),
            UpstreamDecoded::Unknown => self.text.push('?'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CwChannel, CwDecode};

    fn keyed_tone(text: &str, tone_hz: f32, wpm: f32) -> Vec<f32> {
        let dot = (12_000.0 * 1.2 / wpm) as usize;
        let map = |character: char| match character {
            'C' => "-.-.",
            'Q' => "--.-",
            'D' => "-..",
            'E' => ".",
            _ => "",
        };
        let mut samples = Vec::new();
        samples.extend(std::iter::repeat_n(0.0, 12_000));
        for (word_index, word) in text.split_whitespace().enumerate() {
            for (char_index, character) in word.chars().enumerate() {
                for (element_index, element) in map(character).chars().enumerate() {
                    let length = if element == '-' { dot * 3 } else { dot };
                    for index in 0..length {
                        samples.push(
                            (2.0 * std::f32::consts::PI * tone_hz * index as f32 / 12_000.0).sin()
                                * 0.5,
                        );
                    }
                    if element_index + 1 < map(character).len() {
                        samples.extend(std::iter::repeat_n(0.0, dot));
                    }
                }
                if char_index + 1 < word.len() {
                    samples.extend(std::iter::repeat_n(0.0, dot * 3));
                }
            }
            if word_index + 1 < text.split_whitespace().count() {
                samples.extend(std::iter::repeat_n(0.0, dot * 7));
            }
        }
        samples.extend(std::iter::repeat_n(0.0, 12_000));
        samples
    }

    #[test]
    fn selected_channel_decodes_generated_cw() {
        let samples = keyed_tone("CQ DE", 700.0, 20.0);
        let mut channel = CwChannel::new(12_000, 700, 20);
        let mut text = String::new();
        for event in channel.push_samples_with_audio(&samples).0 {
            if let CwDecode::Character(character) = event {
                text.push(character);
            }
        }
        assert!(text.contains('C') || text.contains('Q'));
    }

    #[test]
    fn selected_channel_ignores_silence() {
        let mut channel = CwChannel::new(12_000, 700, 20);
        assert!(channel
            .push_samples_with_audio(&vec![0.0; 12_000 * 3])
            .0
            .is_empty());
    }

    #[test]
    fn monitor_audio_rejects_a_distant_tone() {
        let filtered_rms = |input_tone_hz: f32| {
            let mut channel = CwChannel::new(12_000, 700, 20);
            let samples = (0..12_000)
                .map(|index| {
                    0.3 * (2.0 * std::f32::consts::PI * input_tone_hz * index as f32 / 12_000.0)
                        .sin()
                })
                .collect::<Vec<_>>();
            let filtered = channel.push_samples_with_audio(&samples).1;
            (filtered[2_000..]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
                / (filtered.len() - 2_000) as f32)
                .sqrt()
        };
        assert!(filtered_rms(700.0) > filtered_rms(1_400.0) * 3.0);
    }
}
