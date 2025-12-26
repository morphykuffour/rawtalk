use hidapi::HidApi;
use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};
use std::{fs, thread};

const VID: u16 = 0xC2AB;
const PID: u16 = 0x3939;
const USAGE_PAGE: u16 = 0xFF60;
const USAGE: u16 = 0x61;
const SOCKET: &str = "/tmp/rawtalk.sock";
const MAX_MODE_LEN: usize = 8;
const MIN_SWITCH_INTERVAL_MS: u64 = 10;

fn find_keyboard(api: &HidApi) -> Option<hidapi::HidDevice> {
    api.device_list()
        .find(|d| {
            d.vendor_id() == VID
                && d.product_id() == PID
                && d.usage_page() == USAGE_PAGE
                && d.usage() == USAGE
        })
        .and_then(|d| d.open_device(api).ok())
}

fn send_layer(device: &hidapi::HidDevice, layer: u8) {
    let mut cmd = [0u8; 33];
    cmd[1] = 0x00;
    cmd[2] = layer;

    if device.write(&cmd).is_ok() {
        let mut resp = [0u8; 32];
        if device.read_timeout(&mut resp, 500).unwrap_or(0) > 0 && resp[2] == 0xAA {
            eprintln!("[layer {}] {}", layer, if layer == 0 { "colemak-dh" } else { "qwerty" });
        }
    }
}

fn mode_to_layer(mode: &[u8]) -> u8 {
    match mode {
        b"i" | b"ic" | b"ix" | b"R" | b"Rc" | b"Rx" | b"Rv" | b"Rvc" | b"Rvx" => 0,
        _ => 3,
    }
}

fn handle_client(mut stream: UnixStream, tx: &mpsc::Sender<[u8; MAX_MODE_LEN]>) {
    let mut buf = [0u8; MAX_MODE_LEN];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 { break; }
        let end = buf[..n].iter().position(|&b| b == b'\n' || b == 0).unwrap_or(n);
        if end > 0 && end <= MAX_MODE_LEN {
            let mut msg = [0u8; MAX_MODE_LEN];
            msg[..end].copy_from_slice(&buf[..end]);
            if tx.send(msg).is_err() { break; }
        }
        buf = [0u8; MAX_MODE_LEN];
    }
}

fn cleanup_socket() {
    let _ = fs::remove_file(SOCKET);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    
    // Setup signal handler for graceful shutdown
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            r.store(false, Ordering::SeqCst);
            cleanup_socket();
            std::process::exit(0);
        })?;
    }

    let api = HidApi::new()?;
    let device = find_keyboard(&api).ok_or("keyboard not found")?;
    eprintln!("rawtalk: connected");

    cleanup_socket();
    let listener = UnixListener::bind(SOCKET)?;
    
    #[cfg(unix)]
    fs::set_permissions(SOCKET, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;

    let (tx, rx) = mpsc::channel();

    let tx_clone = tx.clone();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx = tx_clone.clone();
            thread::spawn(move || handle_client(stream, &tx));
        }
    });

    eprintln!("rawtalk: listening on {}", SOCKET);

    let mut last_layer: Option<u8> = None;
    let mut last_switch = Instant::now();

    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(mode_buf) => {
                let end = mode_buf.iter().position(|&b| b == 0).unwrap_or(mode_buf.len());
                let mode = &mode_buf[..end];
                let layer = mode_to_layer(mode);

                let now = Instant::now();
                if last_layer != Some(layer) 
                   && now.duration_since(last_switch).as_millis() >= MIN_SWITCH_INTERVAL_MS as u128 
                {
                    send_layer(&device, layer);
                    last_layer = Some(layer);
                    last_switch = now;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    cleanup_socket();
    Ok(())
}
