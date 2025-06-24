use std::io::Write;

use crossterm::{
    cursor::MoveToColumn,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

#[test]
fn main() -> std::io::Result<()> {
    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    writeln!(stdout, "==> Typing, press Ctrl+C or ESC to exit")?;
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
    write!(stdout, "> ")?;
    stdout.flush()?;

    loop {
        // if event::poll(std::time::Duration::from_millis(500))? {
        //     if let Event::Key(KeyEvent { code, modifiers, kind: _, state: _ }) = event::read()? {
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
            KeyCode::Esc => {
                write!(stdout, "\n")?;
                execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                writeln!(stdout, "ESC pressed, exiting.")?;
                break;
            }
            KeyCode::Left => write!(stdout, "LeftArrow_")?, // ←
            KeyCode::Right => write!(stdout, "RightArrow_")?, // →
            KeyCode::Up => write!(stdout, "UpArrow_")?,     // ↑
            KeyCode::Down => write!(stdout, "DownArrow_")?, // ↓
            KeyCode::Char(c) => {
                write!(stdout, "{}", c)?;
            }
            KeyCode::Enter => {
                write!(stdout, "\n")?;
                execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
                write!(stdout, "> ")?;
            }
            v => {
                write!(stdout, "OTHER_KEY::{v}_")?;
                // execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;
            }
        }

        stdout.flush()?;
    }
    execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))?;

    disable_raw_mode()?;
    println!("<== Quit");
    Ok(())
}
