//! v71 GUI renderer core: deterministic command-buffer based 2D renderer foundation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawCommand {
    Clear(Color),
    FillRect { rect: Rect, color: Color },
    StrokeRect { rect: Rect, color: Color, thickness: u16 },
    Text { x: i32, y: i32, color: Color, content: String },
}

#[derive(Debug, Clone, Default)]
pub struct CommandBuffer {
    commands: Vec<DrawCommand>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn clear(&mut self, color: Color) {
        self.commands.push(DrawCommand::Clear(color));
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(DrawCommand::FillRect { rect, color });
    }

    pub fn stroke_rect(&mut self, rect: Rect, color: Color, thickness: u16) {
        self.commands
            .push(DrawCommand::StrokeRect { rect, color, thickness });
    }

    pub fn text(&mut self, x: i32, y: i32, color: Color, content: impl Into<String>) {
        self.commands.push(DrawCommand::Text {
            x,
            y,
            color,
            content: content.into(),
        });
    }

    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    pub fn deterministic_digest(&self) -> u64 {
        // FNV-1a 64-bit over a stable textual encoding of commands.
        let mut hash: u64 = 0xcbf29ce484222325;
        for cmd in &self.commands {
            let line = format!("{:?}", cmd);
            for b in line.as_bytes() {
                hash ^= u64::from(*b);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash ^= u64::from(b'\n');
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    Software,
    Gpu,
}

#[derive(Debug, Clone)]
pub struct RenderFrame {
    pub width: u32,
    pub height: u32,
    pub backend: RenderBackend,
    pub digest: u64,
    pub command_count: usize,
}

pub fn render_commands(
    width: u32,
    height: u32,
    backend: RenderBackend,
    cb: &CommandBuffer,
) -> RenderFrame {
    RenderFrame {
        width,
        height,
        backend,
        digest: cb.deterministic_digest(),
        command_count: cb.commands().len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_digest_is_stable() {
        let mut a = CommandBuffer::new();
        a.clear(Color::BLACK);
        a.fill_rect(Rect { x: 10, y: 8, w: 40, h: 22 }, Color::rgba(30, 40, 50, 255));
        a.text(12, 14, Color::WHITE, "Vitalis");

        let mut b = CommandBuffer::new();
        b.clear(Color::BLACK);
        b.fill_rect(Rect { x: 10, y: 8, w: 40, h: 22 }, Color::rgba(30, 40, 50, 255));
        b.text(12, 14, Color::WHITE, "Vitalis");

        assert_eq!(a.deterministic_digest(), b.deterministic_digest());
    }

    #[test]
    fn render_frame_tracks_backend_and_command_count() {
        let mut cb = CommandBuffer::new();
        cb.clear(Color::BLACK);
        cb.stroke_rect(
            Rect { x: 1, y: 2, w: 10, h: 11 },
            Color::WHITE,
            2,
        );

        let frame = render_commands(1920, 1080, RenderBackend::Software, &cb);
        assert_eq!(frame.command_count, 2);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.backend, RenderBackend::Software);
    }
}
