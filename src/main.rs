mod app;
mod art;
mod link;
mod social;

use app::{App, ColorChoice, Key, color_list, parse_color, random_preset};
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
        (Mode::Indexed, Some((r, g, b))) => Color::Indexed(art::to_indexed((r, g, b))),
    }
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), Show, LeaveAlternateScreen);
}

fn main() -> io::Result<()> {
    let mut ascii = false;
    let mut solo = false;
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let mut choice = ColorChoice::Preset(0);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let color_arg = if a == "--color" {
            match args.next() {
                Some(v) => Some(v),
                None => {
                    eprintln!(
                        "squid-pet: missing value for --color (choose from: {})",
                        color_list()
                    );
                    std::process::exit(2);
                }
            }
        } else {
            a.strip_prefix("--color=").map(str::to_string)
        };
        if let Some(v) = color_arg {
            match parse_color(&v) {
                Some(c) => choice = c,
                None => {
                    eprintln!(
                        "squid-pet: unknown color '{v}' (choose from: {})",
                        color_list()
                    );
                    std::process::exit(2);
                }
            }
            continue;
        }
        match a.as_str() {
            "--ascii" => ascii = true,
            "--solo" => solo = true,
            "--seed" => seed = args.next().and_then(|s| s.parse().ok()).unwrap_or(seed),
            "-h" | "--help" => {
                println!(
                    "squid-pet - a giant ASCII squid pet\n\nUSAGE: squid-pet [--ascii] [--solo] [--seed N] [--color NAME]\n\n  --color NAME  body color: {}\n  --solo        don't look for other squid-pets in other terminals\n\nKeys: f feed, p pet, space play, c color, v visit a neighbor (or call your guest/visitor home), q/Esc quit\n\nSquid-pets in other terminals on this machine find each other automatically: they\nvisit (about every minute or two, or press v), share fish (f), bubble streams and\nink (space). Repeated f/p/space presses stack up. Use --solo to stay on your own.",
                    color_list()
                );
                return Ok(());
            }
            other => {
                eprintln!("squid-pet: unknown argument '{other}' (try --help)");
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
    let res = run(ascii, seed, mode, choice, solo);
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

fn run(ascii: bool, seed: u64, mode: Mode, choice: ColorChoice, solo: bool) -> io::Result<()> {
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    let size = term.size()?;
    let mut app = App::new(size.width, size.height, seed);
    app.mono = mode == Mode::Mono;
    match choice {
        ColorChoice::Preset(i) => app.init_color(i, false),
        ColorChoice::Random => app.init_color(random_preset(seed), !app.mono),
    }
    let link = if solo { None } else { link::Link::open() };
    if let Some(l) = &link {
        app.enable_social(l.pid);
    }
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
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            if !app.mono && !app.is_away() {
                                app.next_color();
                                force = true
                            }
                        }
                        KeyCode::Char('v') | KeyCode::Char('V') => {
                            app.visit_key();
                            force = true
                        }
                        KeyCode::Char('f') | KeyCode::Char('F') => {
                            app.press(Key::Feed);
                            force = true
                        }
                        KeyCode::Char('p') | KeyCode::Char('P') => {
                            app.press(Key::Pet);
                            force = true
                        }
                        KeyCode::Char(' ') => {
                            app.press(Key::Play);
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
            app.shutdown();
            if let Some(l) = &link {
                for (_, m) in app.outbox.drain(..) {
                    l.broadcast(&m);
                }
            }
            break;
        }
        if let Some(l) = &link {
            for (from, m) in l.recv() {
                app.on_msg(from, m);
            }
        }
        let now = Instant::now();
        if force || now >= next {
            let dt = now.duration_since(last).as_secs_f32().min(0.25);
            last = now;
            app.update(dt);
            if let Some(l) = &link {
                for (to, m) in app.outbox.drain(..) {
                    match to {
                        Some(p) => l.send(p, &m),
                        None => l.broadcast(&m),
                    }
                }
            }
            draw(&mut term, &app, ascii, mode)?;
            if now >= next {
                next = now + FRAME;
            }
        }
    }
    Ok(())
}
