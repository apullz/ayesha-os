use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

/// Spawn the engine's keyboard input thread. Returns a flag that can be used to
/// suspend the thread (set to false) so a foreground applet can take over the
/// terminal. The thread is poll-based so the flag is checked even while idle.
pub fn spawn_input_thread(steer_tx: mpsc::Sender<String>) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(true));
    let flag2 = flag.clone();

    std::thread::spawn(move || {
        use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
        let mut input_buf = String::new();
        loop {
            if !flag2.load(Ordering::Relaxed) {
                break;
            }
            match event::poll(Duration::from_millis(100)) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(_) => break,
            }
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                            if steer_tx.send("\0ctrl-m".to_string()).is_err() { break; }
                        }
                        (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                            if steer_tx.send("\0ctrl-p".to_string()).is_err() { break; }
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if steer_tx.send("\0ctrl-c".to_string()).is_err() { break; }
                        }
                        (KeyCode::Up, KeyModifiers::SHIFT) => {
                            if steer_tx.send("\0shift-up".to_string()).is_err() { break; }
                        }
                        (KeyCode::Down, KeyModifiers::SHIFT) => {
                            if steer_tx.send("\0shift-down".to_string()).is_err() { break; }
                        }
                        (KeyCode::Enter, _) => {
                            let line = input_buf.trim().to_string();
                            if steer_tx.send(line).is_err() { break; }
                            input_buf.clear();
                            print!("\r\n");
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Char(c), _) if c as u8 >= 32 => {
                            input_buf.push(c);
                            print!("{}", c);
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Backspace, _) => {
                            input_buf.pop();
                            print!("\x08 \x08");
                            let _ = std::io::stdout().flush();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Paste(s)) => {
                    for c in s.chars() {
                        if c as u8 >= 32 { input_buf.push(c); print!("{}", c); }
                    }
                    let _ = std::io::stdout().flush();
                }
                _ => {}
            }
        }
    });

    flag
}

/// Stop the input thread (sets its flag to false) and wait long enough for the
/// poll loop to notice and exit.
pub fn suspend_input(flag: &AtomicBool) {
    flag.store(false, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(300));
}
