use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use crate::completion::Completer;
use crate::ui;

/// Spawn the engine's keyboard input thread. Returns a flag that can be used to
/// suspend the thread (set to false) so a foreground applet can take over the
/// terminal. The thread is poll-based so the flag is checked even while idle.
pub fn spawn_input_thread(steer_tx: mpsc::Sender<String>, candidates: Vec<String>, menu_flag: Arc<AtomicBool>) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(true));
    let flag2 = flag.clone();

    std::thread::spawn(move || {
        use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
        let mut input_buf = String::new();
        let mut completer = Completer::new(candidates);

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
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat => {
                    // Global hotkeys (Ctrl+M, Ctrl+P, Ctrl+C)
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0ctrl-m".to_string()).is_err() { break; }
                            continue;
                        }
                        (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0ctrl-p".to_string()).is_err() { break; }
                            continue;
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0ctrl-c".to_string()).is_err() { break; }
                            continue;
                        }
                        // Ctrl+V → try vision from the clipboard image / image path
                        (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0paste-vision".to_string()).is_err() { break; }
                            continue;
                        }
                        _ => {}
                    }

                    // If menu mode is active, route navigation/typing keys as control codes
                    if menu_flag.load(Ordering::Relaxed) {
                        match key.code {
                            KeyCode::Up => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = steer_tx.send("\0menu-up".to_string());
                                }
                            }
                            KeyCode::Down => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = steer_tx.send("\0menu-down".to_string());
                                }
                            }
                            KeyCode::Enter => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = steer_tx.send("\0menu-enter".to_string());
                                }
                            }
                            KeyCode::Esc => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = steer_tx.send("\0menu-esc".to_string());
                                }
                            }
                            KeyCode::Backspace => {
                                if key.kind == KeyEventKind::Press {
                                    let _ = steer_tx.send("\0menu-backspace".to_string());
                                }
                            }
                            KeyCode::Char(c) if c as u8 >= 32
                                && key.kind == KeyEventKind::Press => {
                                    let _ = steer_tx.send(format!("\0menu-char:{}", c));
                                }
                            _ => {}
                        }
                        continue;
                    }

                    // Normal mode key handling
                    match (key.code, key.modifiers) {
                        (KeyCode::Up, KeyModifiers::SHIFT) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0shift-up".to_string()).is_err() { break; }
                        }
                        (KeyCode::Down, KeyModifiers::SHIFT) => {
                            if key.kind == KeyEventKind::Press
                                && steer_tx.send("\0shift-down".to_string()).is_err() { break; }
                        }
                        (KeyCode::Tab, _) => {
                            if key.kind != KeyEventKind::Press { continue; }
                            let prefix = input_buf.trim_start().to_string();
                            let (selected, show_all) = completer.complete(&prefix);
                            if let Some(completed) = selected {
                                // Erase current line
                                let erase_len = input_buf.len();
                                for _ in 0..erase_len {
                                    print!("\x08 \x08");
                                }
                                // Write new buffer
                                input_buf = completed;
                                print!("{}", input_buf);
                                let _ = std::io::stdout().flush();
                                // Show all matches on double-tab
                                if !show_all.is_empty() {
                                    ui::dock_submit_goto();
                                    println!();
                                    for m in &show_all {
                                        print!("  {} ", m);
                                    }
                                    println!();
                                    // Re-draw the (docked) prompt and re-echo buffer
                                    ui::dock_prompt();
                                    print!("{}", input_buf);
                                    let _ = std::io::stdout().flush();
                                }
                            }
                        }
                        (KeyCode::Enter, _) => {
                            if key.kind != KeyEventKind::Press { continue; }
                            let line = input_buf.trim().to_string();
                            // drag-and-drop: an image path dropped on the window
                            // arrives as typed text → auto-route to vision
                            if crate::vision::is_image_path(&line) {
                                if steer_tx.send(format!("\0paste-vision:{}", line)).is_err() { break; }
                            } else if steer_tx.send(line).is_err() { break; }
                            completer.reset();
                            input_buf.clear();
                            if ui::dock_active() {
                                // clear the echoed input line; main moves into the region
                                print!("\r\x1B[2K");
                            } else {
                                print!("\r\n");
                            }
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Char(c), _) if c as u8 >= 32 => {
                            if key.kind != KeyEventKind::Press { continue; }
                            input_buf.push(c);
                            completer.reset();
                            print!("{}", c);
                            let _ = std::io::stdout().flush();
                        }
                        (KeyCode::Backspace, _)
                            if !input_buf.is_empty() => {
                                input_buf.pop();
                                completer.reset();
                                print!("\x08 \x08");
                                let _ = std::io::stdout().flush();
                            }
                        _ => {}
                    }
                }
                Ok(Event::Paste(s)) => {
                    // pasting a copied image file (or image path text) → vision
                    if crate::vision::is_image_path(&s) {
                        let _ = steer_tx.send(format!("\0paste-vision:{}", s));
                    } else {
                        for c in s.chars() {
                            if c as u8 >= 32 { input_buf.push(c); print!("{}", c); }
                        }
                        let _ = std::io::stdout().flush();
                    }
                }
                Ok(Event::Resize(_, _)) => {
                    // re-pin the docked region + status + prompt to the new size
                    ui::dock_refresh();
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
