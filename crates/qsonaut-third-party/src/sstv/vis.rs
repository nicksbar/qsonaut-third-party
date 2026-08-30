pub fn vis_mode_name(vis: u8) -> &'static str {
    match vis {
        0x2c => "Martin M1",
        0x28 => "Martin M2",
        0x3c => "Scottie S1",
        0x38 => "Scottie S2",
        0x4c => "Scottie DX",
        0x08 => "Robot 36",
        0x0c => "Robot 72",
        0x63 => "PD90",
        0x5f => "PD120",
        0x62 => "PD160",
        0x60 => "PD180",
        0x61 => "PD240",
        0x5e => "PD290",
        _ => "unknown SSTV mode",
    }
}
fn decode_vis_header(audio: &[f32], tuning_offset_hz: f32) -> Option<(u8, f32)> {
    let leader1_audio = slice_ms(audio, 40.0, 260.0);
    if dominant_frequency(leader1_audio, tuning_offset_hz) != 1900.0 {
        return None;
    }
    let search_center_hz = (1_900.0 + tuning_offset_hz).round() as i32;
    let actual_leader_hz = peak_frequency(
        leader1_audio,
        (search_center_hz - 150).max(100) as u32,
        (search_center_hz + 150).max(100) as u32,
        10,
    );
    let frequency_offset_hz = actual_leader_hz - 1900.0;
    if (frequency_offset_hz - tuning_offset_hz).abs() > 150.0 {
        return None;
    }
    let classify = |start_ms, end_ms| {
        dominant_frequency(slice_ms(audio, start_ms, end_ms), frequency_offset_hz)
    };
    if classify(301.0, 309.0) != 1200.0 || classify(350.0, 570.0) != 1900.0 {
        return None;
    }
    if classify(614.0, 636.0) != 1200.0 {
        return None;
    }
    let bits_start = LEADER_MS * 2.0 + VIS_BREAK_MS + VIS_BIT_MS;
    let mut vis = 0_u8;
    let mut ones = 0;
    for bit in 0..7 {
        let start = bits_start + bit as f64 * VIS_BIT_MS;
        let frequency = classify(start + 4.0, start + 26.0);
        if frequency == 1100.0 {
            vis |= 1 << bit;
            ones += 1;
        } else if frequency != 1300.0 {
            return None;
        }
    }
    let parity_start = bits_start + 7.0 * VIS_BIT_MS;
    let parity = classify(parity_start + 4.0, parity_start + 26.0);
    let parity_ok = if ones % 2 == 1 {
        parity == 1100.0
    } else {
        parity == 1300.0
    };
    let stop_start = parity_start + VIS_BIT_MS;
    let stop = classify(stop_start + 4.0, stop_start + 26.0);
    (parity_ok && stop == 1200.0).then_some((vis, frequency_offset_hz))
}

struct AutoTargetScan {
    detection: Option<(u8, f32)>,
    strongest_offset_hz: Option<f32>,
    prominence_db: Option<f32>,
}

fn decode_vis_header_auto(audio: &[f32]) -> AutoTargetScan {
    let leader = slice_ms(audio, 40.0, 260.0);
    let mut candidates = ((1_900 + AUTO_TARGET_MIN_OFFSET_HZ)
        ..=(1_900 + AUTO_TARGET_MAX_OFFSET_HZ))
        .step_by(25)
        .map(|frequency_hz| {
            (
                frequency_hz as f32 - 1_900.0,
                tone_power(leader, frequency_hz as f32),
            )
        })
        .collect::<Vec<_>>();
    let mut powers = candidates
        .iter()
        .map(|(_, power)| *power)
        .collect::<Vec<_>>();
    powers.sort_by(f32::total_cmp);
    let median_power = powers.get(powers.len() / 2).copied().unwrap_or_default();
    candidates.sort_by(|left, right| right.1.total_cmp(&left.1));
    let strongest = candidates.first().copied();
    let prominence_db = strongest
        .map(|(_, power)| 10.0 * ((power + f32::EPSILON) / (median_power + f32::EPSILON)).log10());

    let mut tested_offsets = Vec::with_capacity(AUTO_TARGET_CANDIDATES_PER_WINDOW);
    let mut detection = None;
    for (offset_hz, _) in candidates {
        if tested_offsets
            .iter()
            .any(|tested: &f32| (*tested - offset_hz).abs() < 50.0)
        {
            continue;
        }
        tested_offsets.push(offset_hz);
        if let Some(found) = decode_vis_header(audio, offset_hz) {
            detection = Some(found);
            break;
        }
        if tested_offsets.len() == AUTO_TARGET_CANDIDATES_PER_WINDOW {
            break;
        }
    }

    AutoTargetScan {
        detection,
        strongest_offset_hz: strongest.map(|(offset, _)| offset),
        prominence_db,
    }
}
