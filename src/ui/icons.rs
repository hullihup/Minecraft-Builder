//! Small line icons drawn with the painter (no emoji font needed).
//! Every icon is designed on a 24×24 grid and scaled into the target rect.

use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Bolt,
    Message,
    User,
    Globe,
    Branch,
    Scale,
    Sigma,
    Text,
    Database,
    Image,
    Server,
    Layers,
    Puzzle,
    Gear,
    Package,
    Search,
    Plus,
    Minus,
    Trash,
    Play,
    Stop,
    Folder,
    Home,
    Book,
    Note,
    Grid,
    Expand,
    Dots,
    Close,
    Check,
    Warning,
    Copy,
    Star,
    StarFilled,
    Slash,
    Eye,
    PanelLeft,
    PanelBottom,
    PanelRight,
    ChevronRight,
    ChevronDown,
    ArrowUp,
    ArrowDown,
    Code,
    Clock,
    Box,
    Sparkle,
    Upload,
    Refresh,
    Edit,
    Keyboard,
}

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    s: Stroke,
    fill: Color32,
}

impl Pen<'_> {
    fn pt(&self, x: f32, y: f32) -> Pos2 {
        pos2(
            self.r.left() + x / 24.0 * self.r.width(),
            self.r.top() + y / 24.0 * self.r.height(),
        )
    }
    fn k(&self) -> f32 {
        self.r.width() / 24.0
    }
    fn line(&self, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        self.p.add(Shape::line(v, self.s));
    }
    fn closed(&self, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        self.p.add(Shape::closed_line(v, self.s));
    }
    fn filled(&self, pts: &[(f32, f32)]) {
        let v: Vec<Pos2> = pts.iter().map(|(x, y)| self.pt(*x, *y)).collect();
        self.p
            .add(Shape::convex_polygon(v, self.fill, Stroke::NONE));
    }
    fn circle(&self, x: f32, y: f32, r: f32) {
        self.p.circle_stroke(self.pt(x, y), r * self.k(), self.s);
    }
    fn dot(&self, x: f32, y: f32, r: f32) {
        self.p.circle_filled(self.pt(x, y), r * self.k(), self.fill);
    }
    fn rect(&self, x0: f32, y0: f32, x1: f32, y1: f32, rad: f32) {
        let r = Rect::from_min_max(self.pt(x0, y0), self.pt(x1, y1));
        self.p
            .rect_stroke(r, rad * self.k(), self.s, egui::StrokeKind::Middle);
    }
    fn arc(&self, cx: f32, cy: f32, r: f32, a0: f32, a1: f32) {
        let n = 16;
        let pts: Vec<(f32, f32)> = (0..=n)
            .map(|i| {
                let a = (a0 + (a1 - a0) * i as f32 / n as f32).to_radians();
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        self.line(&pts);
    }
}

/// Paints `icon` into `rect` with `color`.
pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let w = (rect.width() / 24.0 * 1.9).clamp(1.1, 2.6);
    let p = Pen {
        p: painter,
        r: rect,
        s: Stroke::new(w, color),
        fill: color,
    };
    use Icon::*;
    match icon {
        Bolt => p.closed(&[
            (13.0, 2.0),
            (4.0, 14.0),
            (11.0, 14.0),
            (10.0, 22.0),
            (20.0, 9.5),
            (13.0, 9.5),
        ]),
        Message => {
            p.closed(&[
                (4.0, 5.0),
                (20.0, 5.0),
                (20.0, 16.0),
                (10.0, 16.0),
                (5.0, 20.0),
                (6.0, 16.0),
                (4.0, 16.0),
            ]);
            p.line(&[(8.0, 9.5), (16.0, 9.5)]);
            p.line(&[(8.0, 12.5), (13.0, 12.5)]);
        }
        User => {
            p.circle(12.0, 8.0, 4.0);
            p.arc(12.0, 21.0, 7.5, 200.0, 340.0);
        }
        Globe => {
            p.circle(12.0, 12.0, 9.0);
            p.line(&[(3.0, 12.0), (21.0, 12.0)]);
            p.arc(12.0, 12.0, 9.0, 0.0, 0.0);
            let k = |a: f32| a;
            let _ = k;
            p.line(&[
                (12.0, 3.0),
                (9.0, 7.0),
                (8.0, 12.0),
                (9.0, 17.0),
                (12.0, 21.0),
            ]);
            p.line(&[
                (12.0, 3.0),
                (15.0, 7.0),
                (16.0, 12.0),
                (15.0, 17.0),
                (12.0, 21.0),
            ]);
        }
        Branch => {
            p.circle(6.0, 5.0, 2.2);
            p.circle(6.0, 19.0, 2.2);
            p.circle(18.0, 8.0, 2.2);
            p.line(&[(6.0, 7.2), (6.0, 16.8)]);
            p.line(&[(18.0, 10.2), (18.0, 12.0), (14.0, 15.0), (7.8, 17.5)]);
        }
        Scale => {
            p.line(&[(12.0, 3.0), (12.0, 21.0)]);
            p.line(&[(7.0, 21.0), (17.0, 21.0)]);
            p.line(&[(4.0, 7.0), (20.0, 7.0)]);
            p.line(&[(4.0, 7.0), (1.5, 13.0), (6.5, 13.0), (4.0, 7.0)]);
            p.line(&[(20.0, 7.0), (17.5, 13.0), (22.5, 13.0), (20.0, 7.0)]);
        }
        Sigma => p.line(&[
            (18.0, 5.0),
            (6.0, 5.0),
            (13.0, 12.0),
            (6.0, 19.0),
            (18.0, 19.0),
        ]),
        Text => {
            p.line(&[(5.0, 6.0), (5.0, 4.0), (19.0, 4.0), (19.0, 6.0)]);
            p.line(&[(12.0, 4.0), (12.0, 20.0)]);
            p.line(&[(9.0, 20.0), (15.0, 20.0)]);
        }
        Database => {
            p.arc(12.0, 6.0, 7.5, 0.0, 360.0);
            p.line(&[(4.5, 6.0), (4.5, 18.0)]);
            p.line(&[(19.5, 6.0), (19.5, 18.0)]);
            p.arc(12.0, 12.0, 7.5, 0.0, 180.0);
            p.arc(12.0, 18.0, 7.5, 0.0, 180.0);
        }
        Image => {
            p.rect(3.0, 4.0, 21.0, 20.0, 2.0);
            p.circle(8.5, 9.0, 1.8);
            p.line(&[
                (3.0, 17.0),
                (9.0, 12.0),
                (13.0, 15.5),
                (16.0, 13.0),
                (21.0, 17.0),
            ]);
        }
        Server => {
            p.rect(3.0, 3.5, 21.0, 10.5, 2.0);
            p.rect(3.0, 13.5, 21.0, 20.5, 2.0);
            p.dot(7.0, 7.0, 1.1);
            p.dot(7.0, 17.0, 1.1);
        }
        Layers => {
            p.closed(&[(12.0, 3.0), (21.0, 8.0), (12.0, 13.0), (3.0, 8.0)]);
            p.line(&[(3.0, 12.5), (12.0, 17.5), (21.0, 12.5)]);
            p.line(&[(3.0, 16.5), (12.0, 21.5), (21.0, 16.5)]);
        }
        Puzzle => {
            p.closed(&[
                (4.0, 8.0),
                (9.0, 8.0),
                (9.0, 6.0),
                (11.0, 4.0),
                (13.0, 6.0),
                (13.0, 8.0),
                (18.0, 8.0),
                (18.0, 12.0),
                (20.0, 12.0),
                (21.5, 14.0),
                (20.0, 16.0),
                (18.0, 16.0),
                (18.0, 20.0),
                (4.0, 20.0),
            ]);
        }
        Gear => {
            p.circle(12.0, 12.0, 3.0);
            for i in 0..8 {
                let a = (i as f32 * 45.0).to_radians();
                let (c, s) = (a.cos(), a.sin());
                p.line(&[
                    (12.0 + c * 6.0, 12.0 + s * 6.0),
                    (12.0 + c * 9.0, 12.0 + s * 9.0),
                ]);
            }
            p.circle(12.0, 12.0, 6.5);
        }
        Package => {
            p.closed(&[
                (12.0, 2.5),
                (20.5, 7.0),
                (20.5, 17.0),
                (12.0, 21.5),
                (3.5, 17.0),
                (3.5, 7.0),
            ]);
            p.line(&[(3.5, 7.0), (12.0, 11.5), (20.5, 7.0)]);
            p.line(&[(12.0, 11.5), (12.0, 21.5)]);
        }
        Search => {
            p.circle(10.5, 10.5, 6.5);
            p.line(&[(15.5, 15.5), (21.0, 21.0)]);
        }
        Plus => {
            p.line(&[(12.0, 5.0), (12.0, 19.0)]);
            p.line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Minus => p.line(&[(5.0, 12.0), (19.0, 12.0)]),
        Trash => {
            p.line(&[(4.0, 6.5), (20.0, 6.5)]);
            p.line(&[(9.0, 6.5), (9.0, 3.5), (15.0, 3.5), (15.0, 6.5)]);
            p.closed(&[(6.0, 6.5), (7.0, 20.5), (17.0, 20.5), (18.0, 6.5)]);
            p.line(&[(10.0, 10.0), (10.0, 17.0)]);
            p.line(&[(14.0, 10.0), (14.0, 17.0)]);
        }
        Play => p.filled(&[(7.0, 4.0), (19.0, 12.0), (7.0, 20.0)]),
        Stop => p.filled(&[(6.0, 6.0), (18.0, 6.0), (18.0, 18.0), (6.0, 18.0)]),
        Folder => p.closed(&[
            (3.0, 6.0),
            (9.0, 6.0),
            (11.0, 8.0),
            (21.0, 8.0),
            (21.0, 19.0),
            (3.0, 19.0),
        ]),
        Home => {
            p.line(&[(3.0, 11.5), (12.0, 3.5), (21.0, 11.5)]);
            p.line(&[(5.5, 9.5), (5.5, 20.0), (18.5, 20.0), (18.5, 9.5)]);
            p.line(&[(10.0, 20.0), (10.0, 14.0), (14.0, 14.0), (14.0, 20.0)]);
        }
        Book => {
            p.closed(&[
                (12.0, 6.0),
                (4.0, 4.0),
                (4.0, 18.0),
                (12.0, 20.0),
                (20.0, 18.0),
                (20.0, 4.0),
            ]);
            p.line(&[(12.0, 6.0), (12.0, 20.0)]);
        }
        Note => {
            p.closed(&[
                (5.0, 3.0),
                (15.0, 3.0),
                (19.0, 7.0),
                (19.0, 21.0),
                (5.0, 21.0),
            ]);
            p.line(&[(15.0, 3.0), (15.0, 7.0), (19.0, 7.0)]);
            p.line(&[(8.5, 12.0), (15.5, 12.0)]);
            p.line(&[(8.5, 16.0), (13.5, 16.0)]);
        }
        Grid => {
            for v in [9.0, 15.0] {
                p.line(&[(v, 3.0), (v, 21.0)]);
                p.line(&[(3.0, v), (21.0, v)]);
            }
        }
        Expand => {
            p.line(&[(3.5, 9.0), (3.5, 3.5), (9.0, 3.5)]);
            p.line(&[(15.0, 3.5), (20.5, 3.5), (20.5, 9.0)]);
            p.line(&[(20.5, 15.0), (20.5, 20.5), (15.0, 20.5)]);
            p.line(&[(9.0, 20.5), (3.5, 20.5), (3.5, 15.0)]);
        }
        Dots => {
            p.dot(5.5, 12.0, 1.6);
            p.dot(12.0, 12.0, 1.6);
            p.dot(18.5, 12.0, 1.6);
        }
        Close => {
            p.line(&[(6.0, 6.0), (18.0, 18.0)]);
            p.line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Check => p.line(&[(4.5, 12.5), (9.5, 17.5), (19.5, 6.5)]),
        Warning => {
            p.closed(&[(12.0, 3.0), (22.0, 20.0), (2.0, 20.0)]);
            p.line(&[(12.0, 9.0), (12.0, 14.0)]);
            p.dot(12.0, 17.0, 1.2);
        }
        Copy => {
            p.rect(8.0, 8.0, 20.0, 20.0, 2.0);
            p.line(&[(4.0, 16.0), (4.0, 4.0), (16.0, 4.0)]);
        }
        Star | StarFilled => {
            let pts: Vec<(f32, f32)> = (0..10)
                .map(|i| {
                    let a = (-90.0 + i as f32 * 36.0f32).to_radians();
                    let r = if i % 2 == 0 { 9.5 } else { 4.2 };
                    (12.0 + r * a.cos(), 12.5 + r * a.sin())
                })
                .collect();
            if icon == StarFilled {
                // Fill as triangles from the center.
                for i in 0..10 {
                    let a = pts[i];
                    let b = pts[(i + 1) % 10];
                    p.filled(&[(12.0, 12.5), a, b]);
                }
            } else {
                p.closed(&pts);
            }
        }
        Slash => {
            p.rect(3.0, 3.0, 21.0, 21.0, 4.0);
            p.line(&[(14.5, 7.0), (9.5, 17.0)]);
        }
        Eye => {
            p.line(&[
                (2.0, 12.0),
                (6.0, 7.0),
                (12.0, 5.0),
                (18.0, 7.0),
                (22.0, 12.0),
                (18.0, 17.0),
                (12.0, 19.0),
                (6.0, 17.0),
                (2.0, 12.0),
            ]);
            p.circle(12.0, 12.0, 3.0);
        }
        PanelLeft | PanelBottom | PanelRight => {
            p.rect(3.0, 4.0, 21.0, 20.0, 2.0);
            match icon {
                PanelLeft => p.filled(&[(3.5, 4.5), (9.0, 4.5), (9.0, 19.5), (3.5, 19.5)]),
                PanelRight => p.filled(&[(15.0, 4.5), (20.5, 4.5), (20.5, 19.5), (15.0, 19.5)]),
                _ => p.filled(&[(3.5, 14.0), (20.5, 14.0), (20.5, 19.5), (3.5, 19.5)]),
            }
        }
        ChevronRight => p.line(&[(9.0, 5.0), (16.0, 12.0), (9.0, 19.0)]),
        ChevronDown => p.line(&[(5.0, 9.0), (12.0, 16.0), (19.0, 9.0)]),
        ArrowUp => {
            p.line(&[(12.0, 20.0), (12.0, 4.0)]);
            p.line(&[(6.0, 10.0), (12.0, 4.0), (18.0, 10.0)]);
        }
        ArrowDown => {
            p.line(&[(12.0, 4.0), (12.0, 20.0)]);
            p.line(&[(6.0, 14.0), (12.0, 20.0), (18.0, 14.0)]);
        }
        Code => {
            p.line(&[(8.0, 6.0), (2.5, 12.0), (8.0, 18.0)]);
            p.line(&[(16.0, 6.0), (21.5, 12.0), (16.0, 18.0)]);
            p.line(&[(13.5, 4.0), (10.5, 20.0)]);
        }
        Clock => {
            p.circle(12.0, 12.0, 9.0);
            p.line(&[(12.0, 6.5), (12.0, 12.0), (16.0, 14.0)]);
        }
        Box => {
            p.closed(&[(12.0, 3.0), (20.5, 7.5), (12.0, 12.0), (3.5, 7.5)]);
            p.line(&[
                (3.5, 7.5),
                (3.5, 16.5),
                (12.0, 21.0),
                (20.5, 16.5),
                (20.5, 7.5),
            ]);
            p.line(&[(12.0, 12.0), (12.0, 21.0)]);
        }
        Sparkle => {
            p.closed(&[
                (12.0, 2.5),
                (14.0, 10.0),
                (21.5, 12.0),
                (14.0, 14.0),
                (12.0, 21.5),
                (10.0, 14.0),
                (2.5, 12.0),
                (10.0, 10.0),
            ]);
        }
        Upload => {
            p.line(&[(12.0, 16.0), (12.0, 3.5)]);
            p.line(&[(7.0, 8.5), (12.0, 3.5), (17.0, 8.5)]);
            p.line(&[(4.0, 15.0), (4.0, 20.0), (20.0, 20.0), (20.0, 15.0)]);
        }
        Refresh => {
            p.arc(12.0, 12.0, 8.0, -60.0, 230.0);
            p.line(&[(16.0, 3.5), (16.0, 6.5), (19.0, 6.5)]);
        }
        Edit => {
            p.closed(&[
                (15.5, 4.0),
                (20.0, 8.5),
                (9.0, 19.5),
                (4.0, 20.0),
                (4.5, 15.0),
            ]);
            p.line(&[(13.0, 6.5), (17.5, 11.0)]);
        }
        Keyboard => {
            p.rect(2.5, 6.0, 21.5, 18.0, 2.0);
            for x in [6.0, 10.0, 14.0, 18.0] {
                p.dot(x, 10.0, 0.9);
            }
            p.line(&[(7.0, 14.5), (17.0, 14.5)]);
        }
    }
}

/// Allocates space and paints an icon.
pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    paint(ui.painter(), r, icon, color);
    resp
}

/// Square icon-only button.
pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tip: &str) -> Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let hov = resp.hovered();
    if hov {
        ui.painter()
            .rect_filled(r, 5.0, Color32::from_white_alpha(14));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let col = if hov {
        crate::theme::TEXT
    } else {
        crate::theme::TEXT_WEAK
    };
    paint(ui.painter(), r.shrink(size * 0.2), icon, col);
    if tip.is_empty() {
        resp
    } else {
        resp.on_hover_text(tip)
    }
}

/// Bordered button with an icon and text, like "+ Add option".
pub fn button(ui: &mut Ui, icon: Option<Icon>, text: &str) -> Response {
    styled_button(ui, icon, text, None)
}

pub fn styled_button(
    ui: &mut Ui,
    icon: Option<Icon>,
    text: &str,
    fill: Option<(Color32, Color32)>,
) -> Response {
    let font = egui::FontId::proportional(13.5);
    let (bg, fg) = fill.unwrap_or((Color32::from_rgb(0x26, 0x28, 0x2D), crate::theme::TEXT));
    let galley = ui.painter().layout_no_wrap(text.to_string(), font, fg);
    let icon_w = if icon.is_some() { 20.0 } else { 0.0 };
    let size = vec2(galley.size().x + icon_w + 24.0, 30.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let hov = resp.hovered();
    let fill_c = if hov && fill.is_none() {
        Color32::from_rgb(0x30, 0x33, 0x39)
    } else if hov {
        bg.gamma_multiply(1.15)
    } else {
        bg
    };
    let stroke = if fill.is_some() {
        Stroke::NONE
    } else {
        Stroke::new(1.0, Color32::from_rgb(0x44, 0x47, 0x4E))
    };
    ui.painter()
        .rect(r, 6.0, fill_c, stroke, egui::StrokeKind::Inside);
    let mut x = r.left() + 12.0;
    if let Some(i) = icon {
        paint(
            ui.painter(),
            Rect::from_center_size(pos2(x + 7.0, r.center().y), Vec2::splat(15.0)),
            i,
            fg,
        );
        x += icon_w;
    }
    ui.painter()
        .galley(pos2(x, r.center().y - galley.size().y / 2.0), galley, fg);
    if hov {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}
