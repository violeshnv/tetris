use super::block;

use std::{
    io::{Stdout, Write},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use terminal::{error::Result, Action, Color, Event, Retrieved, Terminal, Value};

const EMPTY: &'static str = "  ";
const BLOCK: &'static str = "██";

const VERTICAL_BAR: char = '┃';
// const HORIZONTAL_BAR: char = '━';
const TOP_LEFT_CORNER: char = '┏';
const TOP_RIGHT_CORNER: char = '┓';
const BOTTOM_LEFT_CORNER: char = '┗';
const BOTTOM_RIGHT_CORNER: char = '┛';

fn horizontal_line(count: usize, front: char, back: char) -> String {
    format!("{}{:━^count$}{}", front, "", back)
}

pub struct Painter {
    stdout: Mutex<terminal::Terminal<Stdout>>,
    color: Mutex<Color>,
}

impl Drop for Painter {
    fn drop(&mut self) {
        self.leave_terminal().unwrap();
    }
}

impl Painter {
    pub fn new() -> Self {
        let painter = Painter {
            stdout: Mutex::new(terminal::stdout()),
            color: Mutex::new(Color::Reset),
        };
        painter.enter_terminal().unwrap();
        painter
    }

    /// (top, bottom, left, right)
    pub fn clear(&self, boarders: (u16, u16, u16, u16)) -> Result<()> {
        let (top, bottom, left, right) = boarders;
        let (col, row) = ((right - left + 1) as usize, (bottom - top + 1) as usize);
        debug_assert!(top < bottom && left < right);
        self.multiple_writeln_at(
            Color::Reset,
            (left, top),
            std::iter::repeat(format!("{:col$}", "").as_bytes()).take(row),
        )?;
        Ok(())
    }

    fn stdout_lock(&self) -> MutexGuard<'_, Terminal<Stdout>> {
        self.stdout.lock().unwrap()
    }

    pub fn clear_all(&self) -> Result<()> {
        self.stdout_lock()
            .act(Action::ClearTerminal(terminal::Clear::All))?;
        Ok(())
    }

    pub fn flush(&self) -> Result<()> {
        self.stdout.lock().unwrap().flush()?;
        Ok(())
    }

    pub fn get_size(&self) -> Result<(u16, u16)> {
        if let Retrieved::TerminalSize(col, row) =
            self.stdout.lock().unwrap().get(Value::TerminalSize)?
        {
            Ok((col, row))
        } else {
            panic!("shouldn't reach here");
            // Err(ErrorKind::ActionNotSupported(String::new(
            //     "Failed to fetch size",
            // )))
        }
    }

    pub fn get_event(&self) -> Result<Option<Event>> {
        // Duration::from_secs(0) for non-blocking check
        if let Retrieved::Event(event) = self
            .stdout_lock()
            .get(Value::Event(Some(Duration::from_secs(0))))?
        {
            Ok(event)
        } else {
            panic!("shouldn't reach here");
            // Err(ErrorKind::ActionNotSupported(String::new(
            //     "Failed to get event",
            // )))
        }
    }

    pub fn enter_terminal(&self) -> Result<()> {
        let lock = self.stdout_lock();
        lock.batch(terminal::Action::EnterAlternateScreen)?;
        lock.batch(terminal::Action::HideCursor)?;
        lock.batch(terminal::Action::DisableMouseCapture)?;
        lock.batch(terminal::Action::EnableRawMode)?;
        lock.batch(terminal::Action::ClearTerminal(terminal::Clear::All))?;

        lock.flush_batch()?;
        Ok(())
    }

    pub fn leave_terminal(&self) -> Result<()> {
        let lock = self.stdout_lock();

        lock.batch(terminal::Action::LeaveAlternateScreen)?;
        lock.batch(terminal::Action::EnableMouseCapture)?;
        lock.batch(terminal::Action::ShowCursor)?;
        lock.batch(terminal::Action::DisableRawMode)?;
        lock.batch(terminal::Action::ResetColor)?;

        lock.flush_batch()?;
        Ok(())
    }

    pub fn draw_multiple_color_block_at(
        &self,
        left_bottom: (u16, u16),
        points: impl Iterator<Item = (Color, block::Point)>,
    ) -> Result<()> {
        for point in points {
            let (color, block::Point { x, y }) = point;
            let (x, y) = (x * 2, y);

            debug_assert!(left_bottom.0 as isize + x >= 0);
            debug_assert!(left_bottom.1 as isize - y >= 0);

            let pos = (
                (left_bottom.0 as isize + x) as u16,
                (left_bottom.1 as isize - y) as u16,
            );

            self.write_at(
                color,
                pos,
                if let Color::Reset = color {
                    EMPTY
                } else {
                    BLOCK
                }
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    pub fn draw_multiple_block_at<'a>(
        &self,
        color: Color,
        left_bottom: (u16, u16),
        points: impl Iterator<Item = &'a block::Point>,
    ) -> Result<()> {
        for point in points {
            let block::Point { x, y } = point;
            let (x, y) = (x * 2, y);

            debug_assert!(left_bottom.0 as isize + x >= 0);
            debug_assert!(left_bottom.1 as isize - y >= 0);

            let pos = (
                (left_bottom.0 as isize + x) as u16,
                (left_bottom.1 as isize - y) as u16,
            );

            self.write_at(
                color,
                pos,
                if let Color::Reset = color {
                    EMPTY
                } else {
                    BLOCK
                }
                .as_bytes(),
            )?;
        }
        Ok(())
    }

    // pub fn draw_block_at(
    //     &self,
    //     color: Color,
    //     left_bottom: (u16, u16),
    //     point: &block::Point,
    // ) -> Result<()> {
    //     let block::Point { x, y } = point;
    //     let (x, y) = (x * 2, y);

    //     debug_assert!(left_bottom.0 as isize + x >= 0);
    //     debug_assert!(left_bottom.1 as isize - y >= 0);

    //     let pos = (
    //         (left_bottom.0 as isize + x) as u16,
    //         (left_bottom.1 as isize - y) as u16,
    //     );

    //     self.write_at(
    //         color,
    //         pos,
    //         if let Color::Reset = color {
    //             EMPTY
    //         } else {
    //             BLOCK
    //         }
    //         .as_bytes(),
    //     )?;
    //     Ok(())
    // }

    pub fn write_at(&self, color: Color, pos: (u16, u16), buf: &[u8]) -> Result<()> {
        let mut lock = self.stdout_lock();

        let mut color_lock = self.color.lock().unwrap();
        if *color_lock != color {
            *color_lock = color;
            drop(color_lock);
            lock.act(Action::SetForegroundColor(color))?;
        }

        lock.act(Action::MoveCursorTo(pos.0, pos.1))?;
        lock.write_all(buf)?;
        Ok(())
    }

    pub fn multiple_writeln_at<'a>(
        &self,
        color: Color,
        mut pos: (u16, u16),
        mut it: impl Iterator<Item = &'a [u8]>,
    ) -> Result<()> {
        while let Some(buf) = it.next() {
            self.write_at(color, pos, buf)?;
            pos.1 += 1;
        }
        Ok(())
    }

    /// top bottom left right
    pub fn draw_rect(&self, color: Color, boarders: (u16, u16, u16, u16)) -> Result<()> {
        let (top, bottom, left, right) = boarders;
        debug_assert!(top < bottom && left < right);

        let count = (right - left - 1) as usize;
        self.write_at(
            color,
            (left, top),
            horizontal_line(count, TOP_LEFT_CORNER, TOP_RIGHT_CORNER).as_bytes(),
        )?;
        self.write_at(
            color,
            (left, bottom),
            horizontal_line(count, BOTTOM_LEFT_CORNER, BOTTOM_RIGHT_CORNER).as_bytes(),
        )?;

        let vertical_str = VERTICAL_BAR.to_string();
        let vertical_bar = vertical_str.as_bytes();
        for i in top + 1..=bottom - 1 {
            self.write_at(color, (left, i), vertical_bar)?;
            self.write_at(color, (right, i), vertical_bar)?;
        }

        Ok(())
    }
}
