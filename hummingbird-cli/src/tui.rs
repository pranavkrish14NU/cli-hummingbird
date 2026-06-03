use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};

// ── Opsera brand palette ──────────────────────────────────────────────────────
const PURPLE: Color = Color::Rgb {
    r: 124,
    g: 58,
    b: 237,
};
const PURPLE_DIM: Color = Color::Rgb {
    r: 91,
    g: 33,
    b: 182,
};
const GOLD: Color = Color::Rgb {
    r: 245,
    g: 158,
    b: 11,
};
const GOLD_BRIGHT: Color = Color::Rgb {
    r: 252,
    g: 211,
    b: 77,
};
const TEAL: Color = Color::Rgb {
    r: 20,
    g: 184,
    b: 166,
};
const TEAL_DARK: Color = Color::Rgb {
    r: 15,
    g: 118,
    b: 110,
};
const GREEN_WING: Color = Color::Rgb {
    r: 52,
    g: 211,
    b: 153,
};
const WHITE: Color = Color::Rgb {
    r: 240,
    g: 240,
    b: 250,
};
const DIM: Color = Color::Rgb {
    r: 148,
    g: 148,
    b: 163,
};

fn rgb(c: Color) -> String {
    match c {
        Color::Rgb { r, g, b } => format!("38;2;{};{};{}", r, g, b),
        Color::White => "97".to_string(),
        _ => "37".to_string(),
    }
}

fn ansi_line(out: &mut impl Write, code: &str, text: &str) {
    let _ = write!(out, "\x1b[{}m{}\x1b[0m", code, text);
}

fn box_row(out: &mut impl Write, w: usize, text: &str, color: Color, bold: bool) {
    let inner = w - 2;
    let b = if bold { "\x1b[1m" } else { "" };
    let _ = write!(
        out,
        "\x1b[{}m║\x1b[0m{}\x1b[{}m{:<inner$}\x1b[0m\x1b[{}m║\x1b[0m\n",
        rgb(PURPLE),
        b,
        rgb(color),
        text,
        rgb(PURPLE),
        inner = inner
    );
}

fn two_col(
    out: &mut impl Write,
    lw: usize,
    rw: usize,
    left: &str,
    lc: Color,
    bold: bool,
    right: &str,
    rc: Color,
) {
    let l = format!("{:<lw$}", left, lw = lw);
    let r = format!("{:<rw$}", right, rw = rw);
    let b = if bold { "\x1b[1m" } else { "" };
    let _ = write!(
        out,
        "\x1b[{}m║\x1b[0m{}\x1b[{}m{}\x1b[0m\x1b[{}m│\x1b[{}m{}\x1b[0m\x1b[{}m║\x1b[0m\n",
        rgb(PURPLE),
        b,
        rgb(lc),
        l,
        rgb(PURPLE),
        rgb(rc),
        r,
        rgb(PURPLE)
    );
}

// ── Hummingbird pixel art ─────────────────────────────────────────────────────
// Each row: (beak segment, body segment, body color)
// Beak rendered in GOLD, body in the given color.
fn bird_lines() -> &'static [(&'static str, &'static str, Color)] {
    &[
        ("                 ", "  ▄▄████████▄▄              ", TEAL),
        ("                 ", "▄█░░░░░░░░░░░░▀▄            ", TEAL),
        ("─────────────────", "░░(◉)░░░░░░░░░░░█           ", TEAL),
        ("                 ", "▀█░░░░░░░░░░░░░░░▀▄         ", TEAL),
        (
            "      ▄▄▄▄▄▄▄▄▄▄▄",
            "█░░░░░░░░░░░░░░░░░░▀▄      ",
            TEAL_DARK,
        ),
        (
            "     █           ",
            "░░░░░░░░░░░░░░░░░░░░░█      ",
            TEAL_DARK,
        ),
        (
            "      ▀▀▀▀▀▀▀▀▀▀▀",
            "█░░░░░░░░░░░░░░░░░░▄▀      ",
            GREEN_WING,
        ),
        (
            "                 ",
            "  ▀▀█░░░░░░░░░░░░▄▀         ",
            GREEN_WING,
        ),
        (
            "                 ",
            "     ▀▀███░░░░███▀▀         ",
            GREEN_WING,
        ),
        (
            "                 ",
            "        ██░░░░██            ",
            TEAL_DARK,
        ),
        (
            "                 ",
            "        ██    ██            ",
            TEAL_DARK,
        ),
        (
            "                 ",
            "       ██      ██           ",
            PURPLE_DIM,
        ),
        (
            "                 ",
            "      ██        ██          ",
            PURPLE_DIM,
        ),
    ]
}

const CHANGELOG: &[&str] = &[
    "Forge work orders drive agent tasks",
    "Multi-provider: Ollama / OpenAI / Anthropic",
    "Session persistence across conversations",
    "Diff-based code edits with full undo",
    "Sandboxed shell execution with blocklist",
    "Global config via ~/.hummingbird.toml",
];

// ── Trust check ───────────────────────────────────────────────────────────────

pub fn run_trust_check(dir: &str) -> bool {
    terminal::enable_raw_mode().unwrap();
    let mut out = stdout();
    execute!(
        out,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide
    )
    .unwrap();

    let w = terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
        .min(76);

    // Static frame
    ansi_line(
        &mut out,
        &rgb(PURPLE),
        &format!("╔{}╗\n", "═".repeat(w - 2)),
    );
    box_row(&mut out, w, "", DIM, false);
    box_row(&mut out, w, "  Accessing workspace:", GOLD, true);
    box_row(&mut out, w, "", DIM, false);
    box_row(&mut out, w, &format!("  {}", dir), WHITE, false);
    box_row(&mut out, w, "", DIM, false);
    box_row(
        &mut out,
        w,
        "  Quick safety check: Is this a project you created or one you trust?",
        DIM,
        false,
    );
    box_row(
        &mut out,
        w,
        "  (Like your own code, a well-known open source project, or team work.)",
        DIM,
        false,
    );
    box_row(
        &mut out,
        w,
        "  If not, take a moment to review what's in this folder first.",
        DIM,
        false,
    );
    box_row(&mut out, w, "", DIM, false);
    box_row(
        &mut out,
        w,
        "  Hummingbird will be able to read, edit, and execute files here.",
        WHITE,
        false,
    );
    box_row(&mut out, w, "", DIM, false);
    ansi_line(
        &mut out,
        &rgb(PURPLE),
        &format!("╠{}╣\n", "═".repeat(w - 2)),
    );
    out.flush().unwrap();

    let options = ["  ▶  Yes, I trust this folder", "  ▶  No, exit"];
    let mut selected = 0usize;
    let option_row_start = 13u16;
    let inner = w - 2;

    loop {
        for (i, opt) in options.iter().enumerate() {
            queue!(
                out,
                cursor::MoveTo(0, option_row_start + i as u16),
                terminal::Clear(ClearType::CurrentLine)
            )
            .unwrap();
            let _ = write!(out, "\x1b[{}m║\x1b[0m", rgb(PURPLE));
            if i == selected {
                let _ = write!(
                    out,
                    "\x1b[1m\x1b[{}m{:<inner$}\x1b[0m",
                    rgb(GOLD),
                    opt,
                    inner = inner
                );
            } else {
                let _ = write!(
                    out,
                    "\x1b[{}m{:<inner$}\x1b[0m",
                    rgb(DIM),
                    opt,
                    inner = inner
                );
            }
            let _ = write!(out, "\x1b[{}m║\x1b[0m\n", rgb(PURPLE));
        }

        ansi_line(
            &mut out,
            &rgb(PURPLE),
            &format!("╠{}╣\n", "═".repeat(w - 2)),
        );
        let _ = write!(
            out,
            "\x1b[{}m║\x1b[{}m{:<inner$}\x1b[0m\x1b[{}m║\x1b[0m\n",
            rgb(PURPLE),
            rgb(DIM),
            "  Enter to confirm  ·  ↑↓ to move  ·  Esc to cancel",
            rgb(PURPLE),
            inner = inner
        );
        ansi_line(
            &mut out,
            &rgb(PURPLE),
            &format!("╚{}╝\n", "═".repeat(w - 2)),
        );
        out.flush().unwrap();

        if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
            match code {
                KeyCode::Up | KeyCode::Char('k') => selected = selected.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    selected = (selected + 1).min(options.len() - 1)
                }
                KeyCode::Enter => {
                    terminal::disable_raw_mode().unwrap();
                    execute!(out, cursor::Show, ResetColor).unwrap();
                    println!();
                    return selected == 0;
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    terminal::disable_raw_mode().unwrap();
                    execute!(out, cursor::Show, ResetColor).unwrap();
                    println!();
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

    let (term_w, _) = terminal::size().unwrap_or((120, 40));
    let lw: usize = 56;
    let rw: usize = (term_w as usize).saturating_sub(lw + 3).min(50);

    // Top border
    ansi_line(
        &mut out,
        &rgb(PURPLE),
        &format!("╔{}╤{}╗\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Header
    two_col(
        &mut out,
        lw,
        rw,
        &format!("  Hummingbird {}", version),
        GOLD_BRIGHT,
        true,
        "  What's new",
        GOLD,
    );

    // Divider
    ansi_line(
        &mut out,
        &rgb(PURPLE),
        &format!("╠{}╪{}╣\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Welcome message
    two_col(
        &mut out,
        lw,
        rw,
        &format!("  Welcome back, {}!", username),
        WHITE,
        true,
        "",
        DIM,
    );
    two_col(&mut out, lw, rw, "", DIM, false, "", DIM);

    // Bird + changelog
    let bird = bird_lines();
    let total = bird.len().max(CHANGELOG.len() + 1);

    for i in 0..total {
        // Left: bird art (beak in gold, body in its color)
        let _ = write!(out, "\x1b[{}m║\x1b[0m", rgb(PURPLE));

        if let Some((beak, body, color)) = bird.get(i) {
            let beak_part = format!("  {}", beak);
            let body_part = format!(
                "{:<width$}",
                body,
                width = lw.saturating_sub(beak_part.chars().count())
            );
            let _ = write!(
                out,
                "\x1b[{}m{}\x1b[0m\x1b[{}m{}\x1b[0m",
                rgb(GOLD),
                beak_part,
                rgb(*color),
                body_part
            );
        } else {
            let _ = write!(out, "{:<lw$}", "", lw = lw);
        }

        // Right: changelog entry
        let right_str = if i == 0 {
            String::new()
        } else if let Some(entry) = CHANGELOG.get(i - 1) {
            format!("  ▸ {}", entry)
        } else {
            String::new()
        };

        let _ = write!(
            out,
            "\x1b[{}m│\x1b[{}m{:<rw$}\x1b[0m\x1b[{}m║\x1b[0m\n",
            rgb(PURPLE),
            rgb(DIM),
            right_str,
            rgb(PURPLE),
            rw = rw
        );
    }

    // Spacer + branding
    two_col(&mut out, lw, rw, "", DIM, false, "", DIM);
    two_col(
        &mut out,
        lw,
        rw,
        "  Powered by  O P S E R A",
        GOLD_BRIGHT,
        true,
        "",
        DIM,
    );
    two_col(
        &mut out,
        lw,
        rw,
        &format!("  Model  ·  {} via {}", model, provider),
        TEAL,
        false,
        "",
        DIM,
    );
    two_col(
        &mut out,
        lw,
        rw,
        &format!("  Dir    ·  {}", dir),
        DIM,
        false,
        "",
        DIM,
    );
    two_col(&mut out, lw, rw, "", DIM, false, "", DIM);

    // Bottom border
    ansi_line(
        &mut out,
        &rgb(PURPLE),
        &format!("╚{}╧{}╝\n\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Prompt hint line
    let _ = write!(out,
        "\x1b[{}m▶ \x1b[0m\x1b[1m\x1b[{}mauto mode on\x1b[0m  \x1b[{}m·  type a task or /help\x1b[0m\n\n",
        rgb(GOLD), rgb(PURPLE), rgb(DIM));

    out.flush().unwrap();
}
