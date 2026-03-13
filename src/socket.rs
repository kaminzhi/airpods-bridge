pub const SOCKET_PATH: &str = "/tmp/airpods.sock";

use tokio::net::UnixListener;
use tokio::io::{BufReader, AsyncBufReadExt};
use crate::state::SharedState;
use crate::protocol;
use tokio::time::{self, Duration};

pub async fn start_listener(state: SharedState) {
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let st = state.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(&mut stream);
                let mut line = String::new();
                if let Ok(_) = reader.read_line(&mut line).await {
                    handle_command(line.trim(), st).await;
                }
            });
        }
    }
}

async fn handle_command(cmd: &str, state: SharedState) {
    if cmd == "cycle" {
        let (fd, mode, seq, left_on, right_on) = {
            let s = state.lock().unwrap();
            (s.session_fd, s.anc_mode.clone(), s.seq, s.left.level.is_some(), s.right.level.is_some())
        };
        if let (Some(f), Some(m)) = (fd, mode) {
            let can_use_anc = left_on && right_on;
            let target = match m.as_str() {
                "Off" => 2,
                "Noise Cancellation" => 3,
                "Transparency" => 1,
                _ => 2,
            };

            if !(left_on && right_on) {
                eprintln!("[Info] Single earbud detected. ANC toggle might be restricted by hardware.");
            }

            for op in [0x0d, 0x01] {
                let p = protocol::build_anc_payload(target, seq, op);
                let _ = unsafe { libc::send(f, p.as_ptr() as _, p.len(), 0) };
                time::sleep(Duration::from_millis(15)).await;
            }
            state.lock().unwrap().seq = seq.wrapping_add(1);
        }
    }
}
