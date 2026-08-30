fn append_vis_header<F>(vis_code: u8, tone: &mut F, out: &mut Vec<i16>)
where
    F: FnMut(f64, f64, &mut Vec<i16>),
{
    tone(1900.0, LEADER_MS, out);
    tone(1200.0, VIS_BREAK_MS, out);
    tone(1900.0, LEADER_MS, out);
    tone(1200.0, VIS_BIT_MS, out);
    let mut ones = 0;
    for bit in 0..7 {
        let one = vis_code & (1 << bit) != 0;
        ones += usize::from(one);
        tone(if one { 1100.0 } else { 1300.0 }, VIS_BIT_MS, out);
    }
    tone(if ones % 2 == 1 { 1100.0 } else { 1300.0 }, VIS_BIT_MS, out);
    tone(1200.0, VIS_BIT_MS, out);
}
fn crossing_frequency(audio: &[f32]) -> Vec<f32> {
    let mut result = vec![1900.0; audio.len()];
    let mut previous: Option<usize> = None;
    for index in 1..audio.len() {
        if audio[index - 1] <= 0.0 && audio[index] > 0.0 {
            if let Some(last) = previous {
                let period = index - last;
                if period > 0 {
                    let frequency = SAMPLE_RATE_HZ as f32 / period as f32;
                    for value in &mut result[last..=index] {
                        *value = frequency;
                    }
                }
            }
            previous = Some(index);
        }
    }
    result
}

fn dominant_frequency(audio: &[f32], offset_hz: f32) -> f32 {
    let mut best = (0.0_f32, 0.0_f32);
    for frequency in [1100, 1200, 1300, 1900] {
        let power = tone_power(audio, frequency as f32 + offset_hz);
        if power > best.1 {
            best = (frequency as f32, power);
        }
    }
    best.0
}

fn peak_frequency(audio: &[f32], start_hz: u32, end_hz: u32, step_hz: usize) -> f32 {
    (start_hz..=end_hz)
        .step_by(step_hz)
        .map(|frequency| (frequency as f32, tone_power(audio, frequency as f32)))
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(frequency, _)| frequency)
        .unwrap_or(1900.0)
}

fn tone_power(audio: &[f32], frequency_hz: f32) -> f32 {
    let omega = TAU * frequency_hz / SAMPLE_RATE_HZ as f32;
    let coeff = 2.0 * omega.cos();
    let (mut q1, mut q2) = (0.0_f32, 0.0_f32);
    for &sample in audio {
        let q0 = coeff * q1 - q2 + sample;
        q2 = q1;
        q1 = q0;
    }
    q1 * q1 + q2 * q2 - coeff * q1 * q2
}

fn slice_ms(audio: &[f32], start_ms: f64, end_ms: f64) -> &[f32] {
    let start = ms_samples(start_ms).min(audio.len());
    let end = ms_samples(end_ms).min(audio.len()).max(start);
    &audio[start..end]
}

fn ms_samples(ms: f64) -> usize {
    (ms * SAMPLE_RATE_HZ as f64 / 1000.0).round() as usize
}
