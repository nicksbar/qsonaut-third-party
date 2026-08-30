pub fn encode_martin_m1(rgb: &[u8]) -> Result<Vec<i16>, SstvError> {
    encode_martin_m1_with_offset(rgb, 0.0)
}
fn encode_martin_m1_with_offset(
    rgb: &[u8],
    frequency_offset_hz: f64,
) -> Result<Vec<i16>, SstvError> {
    if rgb.len() != WIDTH * HEIGHT * 3 {
        return Err(SstvError::InvalidImage);
    }
    let total = ms_samples(HEADER_MS + IMAGE_MS) + 8;
    let mut out = Vec::with_capacity(total);
    let mut phase = 0.0_f64;
    let mut fractional_samples = 0.0_f64;
    let mut tone = |frequency: f64, duration_ms: f64, out: &mut Vec<i16>| {
        fractional_samples += duration_ms * SAMPLE_RATE_HZ as f64 / 1000.0;
        let count = fractional_samples.floor() as usize;
        fractional_samples -= count as f64;
        let step =
            std::f64::consts::TAU * (frequency + frequency_offset_hz) / SAMPLE_RATE_HZ as f64;
        for _ in 0..count {
            out.push((phase.sin() * 18_000.0).round() as i16);
            phase = (phase + step) % std::f64::consts::TAU;
        }
    };

    append_vis_header(VIS_CODE_MARTIN_M1, &mut tone, &mut out);

    for y in 0..HEIGHT {
        tone(1200.0, SYNC_MS, &mut out);
        tone(1500.0, GAP_MS, &mut out);
        for channel in [1_usize, 2, 0] {
            for x in 0..WIDTH {
                let value = rgb[(y * WIDTH + x) * 3 + channel];
                tone(
                    1500.0 + 800.0 * f64::from(value) / 255.0,
                    CHANNEL_MS / WIDTH as f64,
                    &mut out,
                );
            }
            tone(1500.0, GAP_MS, &mut out);
        }
    }
    Ok(out)
}

pub fn decode_martin_m1(audio: &[f32]) -> Result<DecodedImage, SstvError> {
    let header = ms_samples(HEADER_MS);
    if audio.len() < header + ms_samples(IMAGE_MS) {
        return Err(SstvError::IncompleteAudio);
    }
    let Some((vis, frequency_offset_hz)) = decode_vis_header(&audio[..header], 0.0) else {
        return Err(SstvError::UnsupportedVis);
    };
    if vis != VIS_CODE_MARTIN_M1 {
        return Err(SstvError::UnsupportedVis);
    }
    decode_image_audio(&audio[header..], frequency_offset_hz)
}

fn decode_image_audio(audio: &[f32], frequency_offset_hz: f32) -> Result<DecodedImage, SstvError> {
    if audio.len() < ms_samples(IMAGE_MS) {
        return Err(SstvError::IncompleteAudio);
    }
    let frequencies = crossing_frequency(audio);
    let mut rgb = vec![0_u8; WIDTH * HEIGHT * 3];
    for y in 0..HEIGHT {
        let line_ms = y as f64 * LINE_MS;
        let mut channel_start_ms = line_ms + SYNC_MS + GAP_MS;
        for channel in [1_usize, 2, 0] {
            for x in 0..WIDTH {
                let center_ms = channel_start_ms + (x as f64 + 0.5) * CHANNEL_MS / WIDTH as f64;
                let index = ms_samples(center_ms).min(frequencies.len() - 1);
                let frequency = (frequencies[index] - frequency_offset_hz).clamp(1500.0, 2300.0);
                rgb[(y * WIDTH + x) * 3 + channel] =
                    (((frequency - 1500.0) * 255.0 / 800.0).round() as i32).clamp(0, 255) as u8;
            }
            channel_start_ms += CHANNEL_MS + GAP_MS;
        }
    }
    Ok(DecodedImage {
        width: WIDTH,
        height: HEIGHT,
        rgb,
    })
}
