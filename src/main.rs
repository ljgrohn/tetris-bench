use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use rand::seq::SliceRandom;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

const W: i32 = 10;
const H: i32 = 20;

// ── Pieces ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind { I, O, T, S, Z, J, L }

impl Kind {
    fn rgb(self) -> (u8, u8, u8) {
        match self {
            Kind::I => (0, 220, 255),
            Kind::O => (255, 200, 0),
            Kind::T => (190, 80, 255),
            Kind::S => (60, 220, 90),
            Kind::Z => (255, 70, 80),
            Kind::J => (70, 110, 255),
            Kind::L => (255, 140, 40),
        }
    }
    fn color(self) -> Color {
        let (r, g, b) = self.rgb();
        Color::Rgb { r, g, b }
    }
    /// Spawn cells in a 4×4 (or 3×3 / 2×2) bounding box, rotation state 0.
    fn cells(self) -> [(i32, i32); 4] {
        match self {
            Kind::I => [(0, 1), (1, 1), (2, 1), (3, 1)],
            Kind::O => [(1, 0), (2, 0), (1, 1), (2, 1)],
            Kind::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
            Kind::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Kind::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Kind::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Kind::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
        }
    }
    fn box_size(self) -> i32 {
        match self { Kind::I => 4, Kind::O => 2, _ => 3 }
    }
}

#[derive(Clone, Copy)]
struct Piece {
    kind: Kind,
    x: i32, // board position of bounding-box origin
    y: i32,
    rot: usize, // 0..4
}

impl Piece {
    fn spawn(kind: Kind) -> Self {
        let x = (W - kind.box_size()) / 2;
        Piece { kind, x, y: 0, rot: 0 }
    }
    fn cells(&self) -> [(i32, i32); 4] {
        let mut cells = self.kind.cells();
        let n = self.kind.box_size();
        for _ in 0..self.rot {
            if self.kind == Kind::O { break; }
            for c in cells.iter_mut() {
                let (x, y) = *c;
                *c = (n - 1 - y, x);
            }
        }
        cells.map(|(cx, cy)| (self.x + cx, self.y + cy))
    }
    fn rotated(&self, dir: i32) -> Self {
        let mut p = *self;
        p.rot = (self.rot as i32 + dir).rem_euclid(4) as usize;
        p
    }
}

// ── Game state ──────────────────────────────────────────────────────────

type Board = [[Option<Kind>; W as usize]; H as usize];

struct Game {
    board: Board,
    current: Piece,
    next: Kind,
    bag: Vec<Kind>,
    score: u32,
    lines: u32,
    level: u32,
    game_over: bool,
    paused: bool,
}

impl Game {
    fn new() -> Self {
        let mut g = Game {
            board: [[None; W as usize]; H as usize],
            current: Piece::spawn(Kind::T),
            next: Kind::T,
            bag: Vec::new(),
            score: 0,
            lines: 0,
            level: 1,
            game_over: false,
            paused: false,
        };
        g.current = Piece::spawn(g.draw());
        g.next = g.draw();
        g
    }

    fn draw(&mut self) -> Kind {
        if self.bag.is_empty() {
            self.bag = vec![Kind::I, Kind::O, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L];
            self.bag.shuffle(&mut rand::thread_rng());
        }
        self.bag.pop().unwrap()
    }

    fn collides(&self, p: &Piece) -> bool {
        p.cells().iter().any(|&(x, y)| {
            x < 0 || x >= W || y >= H || (y >= 0 && self.board[y as usize][x as usize].is_some())
        })
    }

    fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let mut p = self.current;
        p.x += dx;
        p.y += dy;
        if self.collides(&p) { false } else { self.current = p; true }
    }

    fn try_rotate(&mut self, dir: i32) {
        // Basic wall kicks: try the rotation, then nudge left/right/up.
        for kick in [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1)] {
            let mut p = self.current.rotated(dir);
            p.x += kick.0;
            p.y += kick.1;
            if !self.collides(&p) { self.current = p; return; }
        }
    }

    fn ghost_y(&self) -> i32 {
        let mut p = self.current;
        while { p.y += 1; !self.collides(&p) } {}
        p.y - 1
    }

    fn lock_piece(&mut self) {
        for (x, y) in self.current.cells() {
            if y < 0 { self.game_over = true; return; }
            self.board[y as usize][x as usize] = Some(self.current.kind);
        }
        self.clear_lines();
        self.current = Piece::spawn(self.next);
        self.next = self.draw();
        if self.collides(&self.current) { self.game_over = true; }
    }

    fn clear_lines(&mut self) {
        let mut cleared = 0u32;
        let mut y = H - 1;
        while y >= 0 {
            if self.board[y as usize].iter().all(|c| c.is_some()) {
                cleared += 1;
                for yy in (1..=y).rev() {
                    self.board[yy as usize] = self.board[(yy - 1) as usize];
                }
                self.board[0] = [None; W as usize];
            } else {
                y -= 1;
            }
        }
        if cleared > 0 {
            self.score += [0, 100, 300, 500, 800][cleared as usize] * self.level;
            self.lines += cleared;
            self.level = 1 + self.lines / 10;
        }
    }

    /// One gravity step. Returns true if the piece locked.
    fn tick(&mut self) -> bool {
        if !self.try_move(0, 1) { self.lock_piece(); true } else { false }
    }

    fn hard_drop(&mut self) {
        let dist = self.ghost_y() - self.current.y;
        self.score += (dist as u32) * 2;
        self.current.y = self.ghost_y();
        self.lock_piece();
    }

    fn drop_interval(&self) -> Duration {
        let ms = (800u64).saturating_sub((self.level as u64 - 1) * 70).max(80);
        Duration::from_millis(ms)
    }
}

// ── Rendering ───────────────────────────────────────────────────────────

const FIELD_BG: Color = Color::Rgb { r: 19, g: 19, b: 27 };
const BORDER: Color = Color::Rgb { r: 98, g: 98, b: 135 };
const DIM: Color = Color::Rgb { r: 130, g: 130, b: 165 };
const GHOST_BG: Color = Color::Rgb { r: 54, g: 54, b: 74 };
const PANEL_W: u16 = 16;

fn shade(c: (u8, u8, u8), f: f32) -> Color {
    let m = |v: u8| (v as f32 * f).clamp(0.0, 255.0) as u8;
    Color::Rgb { r: m(c.0), g: m(c.1), b: m(c.2) }
}

fn centered(text: &str, width: u16) -> String {
    let w = text.chars().count() as u16;
    let left = width.saturating_sub(w) / 2;
    format!("{}{}", " ".repeat(left as usize), text)
}

fn fill_center(text: &str, width: u16) -> String {
    let w = text.chars().count() as u16;
    let left = width.saturating_sub(w) / 2;
    let right = width.saturating_sub(w).saturating_sub(left);
    format!("{}{}{}", " ".repeat(left as usize), text, " ".repeat(right as usize))
}

enum Panel {
    Blank,
    Label(&'static str),
    Value(String),
    Preview(i32),
    Rule,
}

fn render(out: &mut impl Write, g: &Game) -> std::io::Result<()> {
    // Build a display grid: board + ghost + current piece.
    let mut grid: [[Option<Kind>; W as usize]; H as usize] = g.board;
    let mut ghost: [[bool; W as usize]; H as usize] = [[false; W as usize]; H as usize];
    if !g.game_over {
        let gy = g.ghost_y();
        let mut gp = g.current;
        gp.y = gy;
        if gy != g.current.y {
            for (x, y) in gp.cells() {
                if (0..H).contains(&y) { ghost[y as usize][x as usize] = true; }
            }
        }
        for (x, y) in g.current.cells() {
            if (0..H).contains(&y) { grid[y as usize][x as usize] = Some(g.current.kind); }
        }
    }

    // Scale: chunky 4×2 blocks on tall terminals, wide 4×1 on short-wide
    // ones, classic 2×1 otherwise. Borders are generated from the field
    // width so the frame always matches where pieces can land.
    let (cols, rows) = terminal::size().unwrap_or((80, 24));
    let (cw, ch): (u16, u16) = if cols >= 66 && rows >= 47 {
        (4, 2)
    } else if cols >= 66 {
        (4, 1)
    } else {
        (2, 1)
    };
    let bw = W as u16 * cw; // play-field width in terminal columns
    let bh = H as u16 * ch; // play-field height in terminal rows
    let frame_w = bw + 2 + 2 + PANEL_W;
    let footer: Vec<&str> = if frame_w >= 60 {
        vec!["←→ move · ↓ drop · ↑ rotate · SPACE hard drop · P pause · Q quit"]
    } else {
        vec!["←→ move · ↓ drop · ↑ rotate", "SPACE hard drop · P pause · Q quit"]
    };
    let frame_h = 2 + 1 + bh + 1 + footer.len() as u16;
    let ox = cols.saturating_sub(frame_w) / 2;
    let oy = rows.saturating_sub(frame_h) / 2;

    let panel: Vec<Panel> = vec![
        Panel::Label("NEXT"), Panel::Rule,
        Panel::Preview(0), Panel::Preview(1), Panel::Preview(2), Panel::Preview(3),
        Panel::Blank,
        Panel::Label("SCORE"), Panel::Value(g.score.to_string()),
        Panel::Blank,
        Panel::Label("LINES"), Panel::Value(g.lines.to_string()),
        Panel::Blank,
        Panel::Label("LEVEL"), Panel::Value(g.level.to_string()),
    ];

    execute!(out, terminal::Clear(ClearType::All))?;

    // Header: rainbow title over a thin rule.
    execute!(out, cursor::MoveTo(ox, oy), Print(centered("", frame_w)))?;
    execute!(out, cursor::MoveTo(ox + (frame_w.saturating_sub(11)) / 2, oy))?;
    for (i, letter) in "TETRIS".chars().enumerate() {
        let kind = [Kind::I, Kind::O, Kind::T, Kind::S, Kind::J, Kind::L][i];
        execute!(out, SetForegroundColor(kind.color()), SetAttribute(Attribute::Bold), Print(letter))?;
        if i < 5 { execute!(out, Print(" "))?; }
    }
    execute!(out, SetAttribute(Attribute::Reset), ResetColor)?;
    execute!(out, cursor::MoveTo(ox, oy + 1), SetForegroundColor(DIM), Print("─".repeat(frame_w as usize)), ResetColor)?;

    // Top border — exactly as wide as the field below it.
    execute!(out, cursor::MoveTo(ox, oy + 2), SetForegroundColor(BORDER),
        Print(format!("╭{}╮", "─".repeat(bw as usize))), ResetColor)?;
    // Field rows: board row 0 sits directly under the top border and
    // row H-1 directly above the bottom border.
    for y in 0..H as usize {
        for sub in 0..ch {
            let vrow = y as u16 * ch + sub;
            execute!(out, cursor::MoveTo(ox, oy + 3 + vrow),
                SetForegroundColor(BORDER), SetBackgroundColor(FIELD_BG), Print("│"), ResetColor)?;
            for x in 0..W as usize {
                let cell = " ".repeat(cw as usize);
                match grid[y][x] {
                    Some(k) => {
                        let bg = if ch == 2 && sub == 0 {
                            shade(k.rgb(), 1.3)
                        } else if ch == 2 {
                            shade(k.rgb(), 0.8)
                        } else {
                            k.color()
                        };
                        execute!(out, SetBackgroundColor(bg), Print(&cell), ResetColor)?;
                    }
                    None if ghost[y][x] => execute!(out, SetBackgroundColor(GHOST_BG), Print(&cell), ResetColor)?,
                    None => execute!(out, SetBackgroundColor(FIELD_BG), Print(&cell), ResetColor)?,
                }
            }
            execute!(out, SetForegroundColor(BORDER), SetBackgroundColor(FIELD_BG), Print("│"), ResetColor)?;
            match panel.get(vrow as usize) {
                Some(Panel::Label(t)) => execute!(out, Print("  "), SetForegroundColor(DIM),
                    SetAttribute(Attribute::Bold), Print(t), SetAttribute(Attribute::Reset), ResetColor)?,
                Some(Panel::Rule) => execute!(out, Print("  "), SetForegroundColor(DIM),
                    Print("────────"), ResetColor)?,
                Some(Panel::Value(v)) => execute!(out, Print("  "), SetForegroundColor(Color::White),
                    SetAttribute(Attribute::Bold), Print(v), SetAttribute(Attribute::Reset), ResetColor)?,
                Some(Panel::Preview(my)) => {
                    execute!(out, Print("  "))?;
                    let mini = g.next.cells();
                    for mx in 0..4i32 {
                        if mini.contains(&(mx, *my)) {
                            execute!(out, SetBackgroundColor(g.next.color()), Print("  "), ResetColor)?;
                        } else {
                            execute!(out, SetBackgroundColor(FIELD_BG), Print("  "), ResetColor)?;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Bottom border, flush under the last field row.
    execute!(out, cursor::MoveTo(ox, oy + 3 + bh), SetForegroundColor(BORDER),
        Print(format!("╰{}╯", "─".repeat(bw as usize))), ResetColor)?;

    // Footer hints, centered on the whole frame.
    for (i, line) in footer.iter().enumerate() {
        execute!(out, cursor::MoveTo(ox, oy + 4 + bh + i as u16),
            SetForegroundColor(DIM), Print(centered(line, frame_w)), ResetColor)?;
    }

    // Paused / game-over banner across the middle of the frame.
    if g.paused || g.game_over {
        let (title, sub, accent) = if g.game_over {
            ("GAME OVER", format!("score {} · R restart · Q quit", g.score),
             Color::Rgb { r: 255, g: 70, b: 80 })
        } else {
            ("PAUSED", "press P to resume".to_string(), Color::Rgb { r: 255, g: 200, b: 0 })
        };
        let bw2: u16 = 34;
        let inner = bw2 - 2;
        let bx = ox + frame_w.saturating_sub(bw2) / 2;
        let by = oy + frame_h / 2 - 2;
        let banner_bg = Color::Rgb { r: 26, g: 26, b: 40 };
        for i in 0..5u16 {
            execute!(out, cursor::MoveTo(bx, by + i), SetBackgroundColor(banner_bg))?;
            match i {
                0 => execute!(out, SetForegroundColor(BORDER), Print(format!("╭{}╮", "─".repeat(inner as usize))))?,
                4 => execute!(out, SetForegroundColor(BORDER), Print(format!("╰{}╯", "─".repeat(inner as usize))))?,
                2 => execute!(out, SetForegroundColor(BORDER), Print("│"),
                    SetForegroundColor(accent), SetAttribute(Attribute::Bold), Print(fill_center(title, inner)),
                    SetAttribute(Attribute::Reset), SetForegroundColor(BORDER), Print("│"))?,
                3 => execute!(out, SetForegroundColor(BORDER), Print("│"),
                    SetForegroundColor(Color::White), Print(fill_center(&sub, inner)),
                    SetForegroundColor(BORDER), Print("│"))?,
                _ => execute!(out, SetForegroundColor(BORDER), Print(format!("│{}│", " ".repeat(inner as usize))))?,
            }
            execute!(out, ResetColor)?;
        }
    }
    out.flush()
}

// ── Main loop ───────────────────────────────────────────────────────────

fn main() -> std::io::Result<()> {
    let mut out = stdout();
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide, terminal::Clear(ClearType::All))?;

    let result = run(&mut out);

    execute!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn run(out: &mut impl Write) -> std::io::Result<()> {
    let mut game = Game::new();
    let mut last_tick = Instant::now();
    render(out, &game)?;

    loop {
        // Poll briefly so gravity stays smooth while input is responsive.
        let timeout = game.drop_interval().saturating_sub(last_tick.elapsed()).min(Duration::from_millis(16));
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release { continue; }
                // Ctrl-C always quits.
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') { break; }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => break,
                    KeyCode::Char('r') | KeyCode::Char('R') if game.game_over => {
                        game = Game::new();
                        last_tick = Instant::now();
                    }
                    KeyCode::Char('p') | KeyCode::Char('P') if !game.game_over => {
                        game.paused = !game.paused;
                        last_tick = Instant::now();
                    }
                    _ if game.game_over || game.paused => {}
                    KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => { game.try_move(-1, 0); }
                    KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => { game.try_move(1, 0); }
                    KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                        if game.try_move(0, 1) { game.score += 1; }
                        last_tick = Instant::now();
                    }
                    KeyCode::Up | KeyCode::Char('x') | KeyCode::Char('X') => game.try_rotate(1),
                    KeyCode::Char('z') | KeyCode::Char('Z') => game.try_rotate(-1),
                    KeyCode::Char(' ') => { game.hard_drop(); last_tick = Instant::now(); }
                    _ => {}
                }
                render(out, &game)?;
            }
        }

        if !game.game_over && !game.paused && last_tick.elapsed() >= game.drop_interval() {
            game.tick();
            last_tick = Instant::now();
            render(out, &game)?;
        }
    }
    Ok(())
}


