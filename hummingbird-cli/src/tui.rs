use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};

// ── Opsera brand palette ──────────────────────────────────────────────────────
const PURPLE_DEEP: Color = Color::Rgb {
    r: 75,
    g: 55,
    b: 175,
}; // dark body
const PURPLE_MID: Color = Color::Rgb {
    r: 108,
    g: 87,
    b: 210,
}; // main body
const PURPLE_LITE: Color = Color::Rgb {
    r: 160,
    g: 148,
    b: 230,
}; // wing tips / tail
const GOLD: Color = Color::Rgb {
    r: 245,
    g: 180,
    b: 45,
}; // wing highlight
const GOLD_LITE: Color = Color::Rgb {
    r: 252,
    g: 210,
    b: 80,
}; // bright gold
const BORDER_PUR: Color = Color::Rgb {
    r: 124,
    g: 58,
    b: 237,
}; // UI borders
const GOLD_UI: Color = Color::Rgb {
    r: 245,
    g: 158,
    b: 11,
}; // UI gold
const GOLD_BRIGHT: Color = Color::Rgb {
    r: 252,
    g: 211,
    b: 77,
}; // UI bright gold
const TEAL: Color = Color::Rgb {
    r: 20,
    g: 184,
    b: 166,
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

fn ansi_write(out: &mut impl Write, code: &str, text: &str) {
    let _ = write!(out, "\x1b[{}m{}\x1b[0m", code, text);
}

fn box_row(out: &mut impl Write, w: usize, text: &str, color: Color, bold: bool) {
    let inner = w - 2;
    let b = if bold { "\x1b[1m" } else { "" };
    let _ = write!(
        out,
        "\x1b[{}m║\x1b[0m{}\x1b[{}m{:<inner$}\x1b[0m\x1b[{}m║\x1b[0m\n",
        rgb(BORDER_PUR),
        b,
        rgb(color),
        text,
        rgb(BORDER_PUR),
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
        rgb(BORDER_PUR),
        b,
        rgb(lc),
        l,
        rgb(BORDER_PUR),
        rgb(rc),
        r,
        rgb(BORDER_PUR)
    );
}

// ── Hummingbird logo art ──────────────────────────────────────────────────────
// Low-poly geometric style matching the Opsera hummingbird.
// Each row = Vec of (&str segment, Color).
// Bird faces right with wings angled up-left, tail down-left.
fn bird_rows() -> Vec<Vec<(&'static str, Color)>> {
    vec![
        // Upper gold wing — sweeping up
        vec![("                 ◢▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲▲", GOLD_LITE)],
        vec![("              ◢▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓◣", GOLD)],
        vec![("           ◢▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓◣", GOLD)],
        // Gold wing meets purple body
        vec![
            ("        ◢▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓", GOLD),
            ("████████████▓▓▓◣  ", PURPLE_MID),
        ],
        // Beak row — gold beak tip, purple body
        vec![
            ("      ◢▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓", GOLD),
            ("██████████████████◣", PURPLE_DEEP),
            ("──────►", GOLD_LITE),
        ],
        // Gold lower wing start
        vec![
            ("      ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓", GOLD),
            ("██████████████████████", PURPLE_DEEP),
        ],
        vec![
            ("       ◥▓▓▓▓▓▓▓▓▓▓▓▓▓◤", GOLD),
            ("████████████████████████ ", PURPLE_MID),
        ],
        // Pure purple body — widest point
        vec![
            ("              ", DIM),
            ("████████████████████████████████", PURPLE_MID),
        ],
        vec![
            ("               ", DIM),
            ("██████████████████████████████", PURPLE_MID),
        ],
        // Narrowing lower body
        vec![
            ("                ", DIM),
            ("████████████████████████████", PURPLE_MID),
        ],
        vec![
            ("                  ", DIM),
            ("████████████████████████", PURPLE_LITE),
        ],
        // Tail fans out
        vec![
            ("                    ", DIM),
            ("████████████████████", PURPLE_LITE),
        ],
        vec![
            ("                      ", DIM),
            ("████████████████", PURPLE_LITE),
        ],
        vec![
            ("                       ", DIM),
            ("██████", PURPLE_LITE),
            ("    ", DIM),
            ("██████", PURPLE_DEEP),
        ],
        vec![
            ("                      ", DIM),
            ("█████", PURPLE_LITE),
            ("      ", DIM),
            ("█████", PURPLE_DEEP),
        ],
        vec![
            ("                     ", DIM),
            ("████", PURPLE_LITE),
            ("        ", DIM),
            ("████", PURPLE_DEEP),
        ],
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
    let inner = w - 2;

    ansi_write(
        &mut out,
        &rgb(BORDER_PUR),
        &format!("╔{}╗\n", "═".repeat(w - 2)),
    );
    box_row(&mut out, w, "", DIM, false);
    box_row(&mut out, w, "  Accessing workspace:", GOLD_UI, true);
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
    ansi_write(
        &mut out,
        &rgb(BORDER_PUR),
        &format!("╠{}╣\n", "═".repeat(w - 2)),
    );
    out.flush().unwrap();

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
            let _ = write!(out, "\x1b[{}m║\x1b[0m", rgb(BORDER_PUR));
            if i == selected {
                let _ = write!(
                    out,
                    "\x1b[1m\x1b[{}m{:<inner$}\x1b[0m",
                    rgb(GOLD_UI),
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
            let _ = write!(out, "\x1b[{}m║\x1b[0m\n", rgb(BORDER_PUR));
        }
        ansi_write(
            &mut out,
            &rgb(BORDER_PUR),
            &format!("╠{}╣\n", "═".repeat(w - 2)),
        );
        let _ = write!(
            out,
            "\x1b[{}m║\x1b[{}m{:<inner$}\x1b[0m\x1b[{}m║\x1b[0m\n",
            rgb(BORDER_PUR),
            rgb(DIM),
            "  Enter to confirm  ·  ↑↓ to move  ·  Esc to cancel",
            rgb(BORDER_PUR),
            inner = inner
        );
        ansi_write(
            &mut out,
            &rgb(BORDER_PUR),
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
    let lw: usize = 58;
    let rw: usize = (term_w as usize).saturating_sub(lw + 3).min(50);

    // Top border
    ansi_write(
        &mut out,
        &rgb(BORDER_PUR),
        &format!("╔{}╤{}╗\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Header row
    two_col(
        &mut out,
        lw,
        rw,
        &format!("  Hummingbird {}", version),
        GOLD_BRIGHT,
        true,
        "  What's new",
        GOLD_UI,
    );

    // Divider
    ansi_write(
        &mut out,
        &rgb(BORDER_PUR),
        &format!("╠{}╪{}╣\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Welcome
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

    // Bird + changelog rows
    let bird = bird_rows();
    let total = bird.len().max(CHANGELOG.len() + 1);

    for i in 0..total {
        // Left panel: bird art
        let _ = write!(out, "\x1b[{}m║\x1b[0m", rgb(BORDER_PUR));

        let mut left_len = 0usize;
        if let Some(segments) = bird.get(i) {
            for (text, color) in segments {
                let _ = write!(out, "\x1b[{}m{}\x1b[0m", rgb(*color), text);
                left_len += text.chars().count();
            }
        }
        // Pad to lw
        if left_len < lw {
            let _ = write!(out, "{}", " ".repeat(lw - left_len));
        }

        // Right panel: changelog
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
            rgb(BORDER_PUR),
            rgb(DIM),
            right_str,
            rgb(BORDER_PUR),
            rw = rw
        );
    }

    // Spacer + Opsera branding
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
    ansi_write(
        &mut out,
        &rgb(BORDER_PUR),
        &format!("╚{}╧{}╝\n\n", "═".repeat(lw), "═".repeat(rw)),
    );

    // Prompt hint
    let _ = write!(out,
        "\x1b[{}m▶ \x1b[0m\x1b[1m\x1b[{}mauto mode on\x1b[0m  \x1b[{}m·  type a task or /help\x1b[0m\n\n",
        rgb(GOLD_UI), rgb(BORDER_PUR), rgb(DIM)
    );

    out.flush().unwrap();
}
