pub mod app;
pub mod art;
pub mod cat_art;
pub mod link;
pub mod social;
pub use app::cat;

use app::{App, ColorChoice, Key, Kind, Ptr, color_list, parse_color, random_preset};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
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
    let _ = execute!(stdout(), DisableMouseCapture, Show, LeaveAlternateScreen);
}

/// Run the pet, reading `--pet` (squid by default) and the other flags.
pub fn cli() -> io::Result<()> {
    let prog = "terminal-pet";
    let fail = |m: String| -> ! {
        eprintln!("{prog}: {m}");
        std::process::exit(2);
    };
    let mut ascii = false;
    let mut solo = false;
    let mut mouse = true;
    let mut seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let mut pet: Option<Kind> = None;
    let mut color: Option<String> = None;
    let mut breed: Option<String> = None;
    let mut args = std::env::args().skip(1);
    let value = |args: &mut std::iter::Skip<std::env::Args>, a: &str, name: &str, list: String| {
        if let Some(v) = a.strip_prefix(&format!("{name}=")) {
            return Some(v.to_string());
        }
        if a == name {
            return Some(args.next().unwrap_or_else(|| {
                fail(format!("missing value for {name} (choose from: {list})"))
            }));
        }
        None
    };
    while let Some(a) = args.next() {
        if let Some(v) = value(&mut args, &a, "--color", color_list()) {
            color = Some(v);
            continue;
        }
        if let Some(v) = value(&mut args, &a, "--breed", cat::breed_list()) {
            breed = Some(v);
            continue;
        }
        if let Some(v) = value(&mut args, &a, "--pet", "squid, cat".into()) {
            pet = Some(match v.to_ascii_lowercase().as_str() {
                "squid" => Kind::Squid,
                "cat" => Kind::Cat,
                _ => fail(format!("unknown pet '{v}' (choose from: squid, cat)")),
            });
            continue;
        }
        match a.as_str() {
            "--ascii" => ascii = true,
            "--solo" => solo = true,
            "--no-mouse" => mouse = false,
            "--seed" => seed = args.next().and_then(|s| s.parse().ok()).unwrap_or(seed),
            "-h" | "--help" => {
                println!("{}", help(prog));
                return Ok(());
            }
            other => fail(format!("unknown argument '{other}' (try --help)")),
        }
    }
    let pet = pet.unwrap_or(Kind::Squid);
    let mut choice = ColorChoice::Preset(0);
    let mut bchoice: Result<usize, ()> = Ok(0);
    match pet {
        Kind::Squid => {
            if breed.is_some() {
                fail("--breed only applies to --pet cat (use --color for the squid)".into());
            }
            if let Some(v) = &color {
                choice = parse_color(v).unwrap_or_else(|| {
                    fail(format!(
                        "unknown color '{v}' (choose from: {})",
                        color_list()
                    ))
                });
            }
        }
        Kind::Cat => {
            if color.is_some() {
                fail(
                    "--color only applies to the squid (use --breed for the cat; choose from: "
                        .to_string()
                        + &cat::breed_list()
                        + ")",
                );
            }
            if let Some(v) = &breed {
                bchoice = cat::parse_breed(v).unwrap_or_else(|| {
                    fail(format!(
                        "unknown breed '{v}' (choose from: {})",
                        cat::breed_list()
                    ))
                });
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
    if mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    let res = run(ascii, seed, mode, (pet, choice, bchoice), solo);
    restore();
    res
}

fn draw(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &App,
    ascii: bool,
    mode: Mode,
) -> io::Result<Vec<u8>> {
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
    Ok(cv.owner)
}

fn run(
    ascii: bool,
    seed: u64,
    mode: Mode,
    (pet, choice, bchoice): (Kind, ColorChoice, Result<usize, ()>),
    solo: bool,
) -> io::Result<()> {
    let mut term = Terminal::new(CrosstermBackend::new(stdout()))?;
    let size = term.size()?;
    let mut app = App::new(size.width, size.height, seed);
    app.mono = mode == Mode::Mono;
    if pet == Kind::Cat {
        app.init_cat();
        match bchoice {
            Ok(i) => app.init_color(i, false),
            Err(()) => app.init_color(cat::random_breed(seed), true),
        }
    } else {
        match choice {
            ColorChoice::Preset(i) => app.init_color(i, false),
            ColorChoice::Random => app.init_color(random_preset(seed), !app.mono),
        }
    }
    let link = if solo { None } else { link::Link::open() };
    if let Some(l) = &link {
        app.enable_social(l.pid);
    }
    let mut last = Instant::now();
    let mut next = last;
    let o = draw(&mut term, &app, ascii, mode)?;
    app.set_hit(o);
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
                            if (!app.mono || app.is_cat()) && !app.is_away() {
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
                    Event::Mouse(m) => {
                        let ev = match m.kind {
                            MouseEventKind::Down(MouseButton::Left) => Some(Ptr::Down),
                            MouseEventKind::Up(MouseButton::Left) => Some(Ptr::Up),
                            MouseEventKind::Drag(MouseButton::Left) => Some(Ptr::Drag),
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                                Some(Ptr::Scroll)
                            }
                            _ => None,
                        };
                        if let Some(path) = std::env::var_os("SQUID_MOUSE_LOG") {
                            use std::io::Write;
                            if let Ok(mut f) = std::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(path)
                            {
                                let _ = writeln!(
                                    f,
                                    "{:.3} {:?} col={} row={} boops_before={}",
                                    app.t, m.kind, m.column, m.row, app.touch.boop
                                );
                            }
                        }
                        if let Some(ev) = ev {
                            app.pointer(ev, m.column, m.row);
                            force = true;
                        }
                    }
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
            let o = draw(&mut term, &app, ascii, mode)?;
            app.set_hit(o);
            if now >= next {
                next = now + FRAME;
            }
        }
    }
    Ok(())
}

fn help(prog: &str) -> String {
    format!(
        "{prog} - a giant ASCII pet for your terminal: a squid or a cat\n\nUSAGE: {prog} [--pet squid|cat] [--ascii] [--solo] [--no-mouse] [--seed N] [--color NAME | --breed NAME]\n\n  --pet KIND    which pet: squid (default) or cat\n  --color NAME  squid color: {}\n  --breed NAME  cat breed: {}\n  --solo        don't look for other pets in other terminals\n  --no-mouse    don't capture the mouse (keeps terminal text selection)\n\nKeys: f feed (squid) / treat (cat), p pet, space play (squid) / laser dot (cat), c color / breed, v visit a neighbor (or call your guest/visitor home), q/Esc quit\nMouse: click = boop, drag across the pet = stroke, scroll wheel over it = ruffle\n\nPets in other terminals on this machine find each other automatically (a cat and a squid can visit each other): they\nvisit (on their own every 20-30 minutes, with a 10 minute pause after a visit; or press v any time) and share treats, fish, motes, bubbles, ink and pounces. Repeated f/p/space presses stack up. Use --solo to stay on your own.",
        color_list(),
        cat::breed_list()
    )
}
