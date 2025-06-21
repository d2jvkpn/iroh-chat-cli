use std::io::Write;
// use std::time::Duration;

use crossterm::{
    cursor::{MoveToColumn, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

fn main() -> std::io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    let mut buffer = String::new();
    let mut cursor_pos = 0;
    writeln!(stdout, "==> Typing, press Ctrl+C to exit")?;
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

    loop {
        //if event::poll(Duration::from_millis(100))? {
        //    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
        let key_event: KeyEvent = match event::read() {
            Ok(Event::Key(v)) => v,
            _ => continue,
        };

        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                write!(stdout, "\n")?;
                execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                writeln!(stdout, "Ctrl+C pressed, exiting.")?;
                break;
            }
            KeyCode::Char('s') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                // ?? can't use Ctrl+Enter
                write!(stdout, "\n")?;
                execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                writeln!(stdout, "Ctrl+S pressed, sending.")?;
                break;
            }
            KeyCode::Char(c) => {
                buffer.insert(cursor_pos, c);
                cursor_pos += 1;
            }
            KeyCode::Left => {
                if cursor_pos > 0 {
                    cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if cursor_pos < buffer.len() {
                    cursor_pos += 1;
                }
            }
            KeyCode::Backspace => {
                if cursor_pos > 0 {
                    buffer.remove(cursor_pos - 1);
                    cursor_pos -= 1;
                }
            }
            KeyCode::Delete => {
                if cursor_pos < buffer.len() {
                    buffer.remove(cursor_pos);
                }
            }
            KeyCode::Enter => {
                write!(stdout, "\n")?;
                buffer.clear();
                cursor_pos = 0;
            }
            _ => {}
        }

        execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
        write!(stdout, "> {}", buffer)?;
        execute!(stdout, MoveToColumn((cursor_pos + 2) as u16))?;
        stdout.flush()?;
    }

    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    execute!(stdout, Show)?;

    disable_raw_mode()?;
    println!("<== Quit");
    Ok(())
}
