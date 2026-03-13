use crate::state::{AirPodsState, BatteryInfo};

/// Apple Accessory Protocol (AAP) Constants
pub const HANDSHAKE: &[u8] = &[
    0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub const REQUEST_NOTIF: &[u8] = &[0x04, 0x00, 0x04, 0x00, 0x0f, 0x00, 0xff, 0xff, 0xff, 0xff];

const BATT_PREFIX: &[u8] = &[0x04, 0x00, 0x04, 0x00, 0x04, 0x00];

pub fn build_anc_payload(next_mode: u8, seq: u8, opcode: u8) -> Vec<u8> {
    vec![
        0x04, 0x00, 0x04, 0x00, 0x0a, 0x00, opcode, next_mode, seq, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ]
}

pub fn parse_data(data: &[u8], s: &mut AirPodsState) -> bool {
    let mut changed = false;

    if let Some(idx) = data
        .windows(BATT_PREFIX.len())
        .position(|w| w == BATT_PREFIX)
    {
        if let Some(sub) = data.get(idx..) {
            if sub.len() >= 7 {
                let count = sub[6];
                let mut pos = 7;
                for _ in 0..count {
                    if pos + 4 >= sub.len() {
                        break;
                    }
                    let (bt, raw, st) = (sub[pos], sub[pos + 2], sub[pos + 3]);

                    let info = BatteryInfo {
                        level: if raw > 100 { None } else { Some(raw) },
                        charging: (st & 0x01) != 0,
                    };

                    match bt {
                        0x01 | 0x04 => {
                            if s.left.level != info.level || s.left.charging != info.charging {
                                s.left = info;
                                changed = true;
                            }
                        }
                        0x02 => {
                            if s.right.level != info.level || s.right.charging != info.charging {
                                s.right = info;
                                changed = true;
                            }
                        }
                        0x08 => {
                            if s.case.level != info.level || s.case.charging != info.charging {
                                s.case = info;
                                changed = true;
                            }
                        }
                        _ => {}
                    }
                    pos += 5;
                }
            }
        }
    }

    let both_ears_connected = s.left.level.is_some() && s.right.level.is_some();

    if !both_ears_connected {
        if s.anc_mode.is_some() {
            s.anc_mode = None;
            changed = true;
        }
    } else {
        for i in 0..data.len().saturating_sub(9) {
            if &data[i..i + 4] == [0x04, 0x00, 0x04, 0x00] {
                let op = data[i + 6];
                if [0x01, 0x0d, 0x0e].contains(&op) {
                    let m = data[i + 7];
                    s.seq = data[i + 8].wrapping_add(1);

                    let mode = match m {
                        1 => Some("Off".into()),
                        2 => Some("Noise Cancellation".into()),
                        3 => Some("Transparency".into()),
                        4 => Some("Adaptive".into()),
                        _ => None,
                    };

                    if mode.is_some() && s.anc_mode != mode {
                        s.anc_mode = mode;
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}
