use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};

const ORANGE: Color = Color::Rgb {
    r: 255,
    g: 140,
    b: 0,
};
const TEAL: Color = Color::Rgb {
    r: 0,
    g: 200,
    b: 180,
};
const DARK_TEAL: Color = Color::Rgb {
    r: 0,
    g: 160,
    b: 140,
};
const GREEN: Color = Color::Rgb {
    r: 80,
    g: 200,
    b: 120,
};
const GRAY: Color = Color::Rgb {
    r: 120,
    g: 120,
    b: 130,
};
const WHITE: Color = Color::White;
const DIM: Color = Color::Rgb {
    r: 160,
    g: 160,
    b: 170,
};

struct Bird;
impl Bird {
    fn lines() -> &'static [(&'static str, Color)] {
        &[
            ("       ████████            ", TEAL),
            ("     ██░░░░░░░░██          ", TEAL),
            ("    ██░░  ●   ░░██─────────", TEAL),
            ("    ██░░░░░░░░░░█████████  ", TEAL),
            ("    ██░░░░░░░░░░█████████  ", DARK_TEAL),
            ("    ████████████           ", DARK_TEAL),
            ("     ████████              ", GREEN),
            ("    ████  ████             ", GREEN),
            ("   ████    ████            ", GREEN),
            ("  ████      ████           ", GREEN),
            ("  ██          ██           ", DARK_TEAL),
        ]
    }
}

const CHANGELOG: &[&str] = &[
    "Forge work orders drive agent tasks",
    "Multi-provider: Ollama / OpenAI / Anthropic",
    "Session persistence across conversations",
    "Diff-based code edits with undo support",
    "Sandboxed shell execution",
];

// ── Trust check ───────────────────────────────────────────────────────────────

pub fn run_trust_check(dir: &str) -> bool {
    terminal::enable_raw_mode().unwrap();
    let mut out = stdout();
    execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();

    let w = terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
        .min(74);
    let inner = w - 2;

    macro_rules! border {
        ($top:expr, $bot:expr) => {
            queue!(
                out,
                SetForegroundColor(ORANGE),
                Print(format!("╔{}╗\n", "═".repeat(w - 2))),
                ResetColor
            )
            .unwrap();
        };
    }
    macro_rules! row {
        ($text:expr) => {{
            queue!(
                out,
                SetForegroundColor(ORANGE),
                Print("║"),
                ResetColor,
                Print(format!("{:<inner$}", $text, inner = inner)),
                SetForegroundColor(ORANGE),
                Print("║\n"),
                ResetColor
            )
            .unwrap();
        }};
        ($text:expr, $color:expr, $bold:expr) => {{
            queue!(out, SetForegroundColor(ORANGE), Print("║"), ResetColor).unwrap();
            if $bold {
                queue!(out, SetAttribute(Attribute::Bold)).unwrap();
            }
            queue!(
                out,
                SetForegroundColor($color),
                Print(format!("{:<inner$}", $text, inner = inner)),
                ResetColor,
                SetAttribute(Attribute::Reset),
                SetForegroundColor(ORANGE),
                Print("║\n"),
                ResetColor
            )
            .unwrap();
        }};
    }

    queue!(
        out,
        SetForegroundColor(ORANGE),
        Print(format!("╔{}╗\n", "═".repeat(w - 2))),
        ResetColor
    )
    .unwrap();
    row!("");
    row!("  Accessing workspace:", ORANGE, true);
    row!("");
    row!(format!("  {}", dir), WHITE, false);
    row!("");
    row!("  Quick safety check: Is this a project you created or one you trust?");
    row!("  (Your own code, a well-known open source project, or team work.)");
    row!("  If not, take a moment to review what's in this folder first.");
    row!("");
    row!("  Hummingbird will be able to read, edit, and execute files here.");
    row!("");
    queue!(
        out,
        SetForegroundColor(ORANGE),
        Print(format!("╠{}╣\n", "═".repeat(w - 2))),
        ResetColor
    )
    .unwrap();

    let options = ["  ▶  Yes, I trust this folder", "  ▶  No, exit"];
    let mut selected = 0usize;
    let option_row_start = 13u16;

    loop {
        for (i, opt) in options.iter().enumerate() {
            queue!(
                out,
                cursor::MoveTo(0, option_row_start + i as u16),
                terminal::Clear(ClearType::CurrentLine)
            )
            .unwrap();
            queue!(out, SetForegroundColor(ORANGE), Print("║"), ResetColor).unwrap();
            if i == selected {
                queue!(
                    out,
                    SetForegroundColor(GREEN),
                    SetAttribute(Attribute::Bold),
                    Print(format!("{:<inner$}", opt, inner = inner)),
                    ResetColor,
                    SetAttribute(Attribute::Reset)
                )
                .unwrap();
            } else {
                queue!(
                    out,
                    SetForegroundColor(GRAY),
                    Print(format!("{:<inner$}", opt, inner = inner)),
                    ResetColor
                )
                .unwrap();
            }
            queue!(out, SetForegroundColor(ORANGE), Print("║\n"), ResetColor).unwrap();
        }

        queue!(
            out,
            SetForegroundColor(ORANGE),
            Print(format!("╠{}╣\n", "═".repeat(w - 2))),
            ResetColor
        )
        .unwrap();
        queue!(
            out,
            SetForegroundColor(ORANGE),
            Print("║"),
            ResetColor,
            SetForegroundColor(DIM),
            Print(format!(
                "{:<inner$}",
                "  Enter to confirm · ↑↓ to move · Esc to cancel",
                inner = inner
            )),
            ResetColor,
            SetForegroundColor(ORANGE),
            Print("║\n"),
            ResetColor
        )
        .unwrap();
        queue!(
            out,
            SetForegroundColor(ORANGE),
            Print(format!("╚{}╝\n", "═".repeat(w - 2))),
            ResetColor
        )
        .unwrap();

        out.flush().unwrap();

        if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
            match code {
                KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(options.len() - 1)
                }
                KeyCode::Enter => {
                    terminal::disable_raw_mode().unwrap();
                    execute!(out, ResetColor).unwrap();
                    return selected == 0;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    terminal::disable_raw_mode().unwrap();
                    return false;
                }
                _ => {}
            }
        }
    }
}

// ── Welcome screen ────────────────────────────────────────────────────────────

pub fn show_welcome(username: &str, model: &str, provider: &str, dir: &str, version: &str) {
    let mut out = stdout();
    execute!(out, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0)).unwrap();

    let (term_w, _) = terminal::size().unwrap_or((110, 40));
    let lw = 52usize;
    let rw = (term_w as usize).saturating_sub(lw + 3).min(48);

    // Helper: print a two-column border row with pre-built strings
    let row = |out: &mut dyn Write,
               left: &str,
               left_color: Color,
               left_bold: bool,
               right: &str,
               right_color: Color| {
        let l = format!("{:<lw$}", left, lw = lw);
        let r = format!("{:<rw$}", right, rw = rw);
        let _ = out.write_all(format!("\x1b[{}m║\x1b[0m", color_code(ORANGE)).as_bytes());
        if left_bold {
            let _ = out.write_all(b"\x1b[1m");
        }
        let _ = out.write_all(format!("\x1b[{}m{}\x1b[0m", color_code(left_color), l).as_bytes());
        let _ = out.write_all(format!("\x1b[{}m│\x1b[0m", color_code(ORANGE)).as_bytes());
        let _ = out.write_all(format!("\x1b[{}m{}\x1b[0m", color_code(right_color), r).as_bytes());
        let _ = out.write_all(format!("\x1b[{}m║\x1b[0m\n", color_code(ORANGE)).as_bytes());
    };

    let top = format!(
        "\x1b[{}m╔{}╤{}╗\x1b[0m\n",
        color_code(ORANGE),
        "═".repeat(lw),
        "═".repeat(rw)
    );
    let mid = format!(
        "\x1b[{}m╠{}╪{}╣\x1b[0m\n",
        color_code(ORANGE),
        "═".repeat(lw),
        "═".repeat(rw)
    );
    let bot = format!(
        "\x1b[{}m╚{}╧{}╝\x1b[0m\n",
        color_code(ORANGE),
        "═".repeat(lw),
        "═".repeat(rw)
    );
    let div = format!(
        "\x1b[{}m╠{}╣\x1b[0m\n",
        color_code(ORANGE),
        "═".repeat(lw + rw + 1)
    );

    out.write_all(top.as_bytes()).unwrap();

    // Header row
    row(
        &mut out,
        &format!("  Hummingbird {}", version),
        ORANGE,
        true,
        "  What's new",
        ORANGE,
    );

    out.write_all(mid.as_bytes()).unwrap();

    // Welcome row
    row(
        &mut out,
        &format!("  Welcome back, {}!", username),
        WHITE,
        true,
        "",
        DIM,
    );
    row(&mut out, "", DIM, false, "", DIM);

    // Bird + changelog
    let bird = Bird::lines();
    let total = bird.len().max(CHANGELOG.len() + 1);

    for i in 0..total {
        let (bird_text, bird_color) = bird.get(i).copied().unwrap_or(("", TEAL));
        let right_text = if i == 0 {
            String::new()
        } else if let Some(entry) = CHANGELOG.get(i - 1) {
            format!("  ▸ {}", entry)
        } else {
            String::new()
        };

        let l = format!("{:<lw$}", bird_text, lw = lw);
        let r = format!("{:<rw$}", right_text, rw = rw);
        out.write_all(
            format!(
                "\x1b[{}m║\x1b[{}m{}\x1b[0m\x1b[{}m│\x1b[{}m{}\x1b[0m\x1b[{}m║\x1b[0m\n",
                color_code(ORANGE),
                color_code(bird_color),
                l,
                color_code(ORANGE),
                color_code(DIM),
                r,
                color_code(ORANGE),
            )
            .as_bytes(),
        )
        .unwrap();
    }

    row(&mut out, "", DIM, false, "", DIM);
    row(&mut out, "  Powered by  OPSERA", ORANGE, true, "", DIM);
    row(
        &mut out,
        &format!("  Model  ·  {} via {}", model, provider),
        TEAL,
        false,
        "",
        DIM,
    );
    row(
        &mut out,
        &format!("  Dir    ·  {}", dir),
        GRAY,
        false,
        "",
        DIM,
    );
    row(&mut out, "", DIM, false, "", DIM);

    out.write_all(bot.as_bytes()).unwrap();

    // Prompt hint
    out.write_all(
        format!(
            "\x1b[{}m▶ \x1b[0m\x1b[{}mauto mode on  ·  type a task or /help\x1b[0m\n\n",
            color_code(GREEN),
            color_code(DIM)
        )
        .as_bytes(),
    )
    .unwrap();

    out.flush().unwrap();
}

// ── ANSI color code helper ────────────────────────────────────────────────────

fn color_code(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!("38;2;{};{};{}", r, g, b),
        Color::White => "37".to_string(),
        Color::Green => "32".to_string(),
        _ => "37".to_string(),
    }
}
