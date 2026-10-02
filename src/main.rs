mod app;
mod art;

use app::{App, State};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::style::Color;
use std::io::{self, stdout};
use std::time::{Duration, Instant, SystemTime};

const FRAME: Duration = Duration::from_millis(62); // ~16 fps

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    True,
    Indexed,
    Mono,
}

fn color(mode: Mode, c: Option<art::Rgb>) -> Color {
    match (mode, c) {
        (_, None) | (Mode::Mono, _) => Color::Reset,
        (Mode::True, Some((r, g, b))) => Color::Rgb(r, g, b),
        (Mode::Indexed, Some((r, g, b))) => {
            let q = |v: u8| (v as f32 / 255.0 * 5.0).round() as u8;
            Color::Indexed(16 + 36 * q(r) + 6 * q(g) + q(b))
        }
    }
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), Show, LeaveAlternateScreen);
}

fn main() -> io::Result<()> {
    let mut ascii = false;
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--ascii" => ascii = true,
            "--seed" => seed = args.next().and_then(|s| s.parse().ok()).unwrap_or(seed),
            "-h" | "--help" => {
                println!(
                    "squid - a giant ASCII squid pet\n\nUSAGE: squid [--ascii] [--seed N]\n\nKeys: f feed, p pet, space play, q/Esc quit"
                );
                return Ok(());
            }
            other => {
                eprintln!("squid: unknown argument '{other}' (try --help)");
                std::process::exit(2);
            }
        }
    }
    let mode = if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        Mode::Mono
    } else if std::env::var("COLORTERM")
        .is_ok_and(|v| v.contains("truecolor") || v.contains("24bit"))
    {
        Mode::True
    } else {
        Mode::Indexed
    };

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        restore();
        hook(i);
    }));
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    let res = run(ascii, seed, mode);
    restore();
    res
}

fn draw(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &App,
    ascii: bool,
    mode: Mode,
) -> io::Result<()> {
    let cv = art::render(app, ascii);
    term.draw(|f| {
        let area = f.area();
        let buf = f.buffer_mut();
        for y in 0..(area.height as usize).min(cv.h) {
            for x in 0..(area.width as usize).min(cv.w) {
                let c = cv.cells[y * cv.w + x];
                let cell = &mut buf[(x as u16, y as u16)];
                cell.set_char(c.ch);
                cell.set_fg(color(mode, c.fg));
            }
        }
    })?;
    Ok(())
}

fn run(ascii: bool, seed: u64, mode: Mode) -> io::Result<()> {
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    let size = term.size()?;
    let mut app = App::new(size.width, size.height, seed);
    let mut last = Instant::now();
    let mut next = last;
    draw(&mut term, &app, ascii, mode)?;
    while !app.quit {
        let mut force = false;
        let wait = next.saturating_duration_since(Instant::now());
        if event::poll(wait)? {
            loop {
                match event::read()? {
                    Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.quit = true,
                        KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit = true
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            app.start(State::Feeding);
                            force = true
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            app.start(State::Petting);
                            force = true
                        }
                        KeyCode::Char(' ') => {
                            app.start(State::Playing);
                            force = true
                        }
                        _ => {}
                    },
                    Event::Resize(w, h) => {
                        app.resize(w, h);
                        force = true;
                    }
                    _ => {}
                }
                if !event::poll(Duration::ZERO)? {
                    break;
                }
            }
        }
        if app.quit {
            break;
        }
        let now = Instant::now();
        if force || now >= next {
            let dt = now.duration_since(last).as_secs_f32().min(0.25);
            last = now;
            app.update(dt);
            draw(&mut term, &app, ascii, mode)?;
            if now >= next {
                next = now + FRAME;
            }
        }
    }
    Ok(())
}
