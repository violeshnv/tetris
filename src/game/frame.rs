use terminal::Color;

use super::block;
use super::painter;
use super::state;

use std::sync::{
    atomic::{AtomicU16, Ordering},
    Arc,
};

const PAUSE_PRINT: (Color, &'static str, u16, u16) = (
    Color::Rgb(135, 206, 250),
    concat!(
        "██████╗\n",
        "██╔══██╗\n",
        "██████╔╝\n",
        "██╔═══╝ \n",
        "██║\n",
        "╚═╝",
    ),
    8,
    6,
);

const NEXT_BLOCK_FRAME_WIDTH: u16 = (block::POINT_OF_BLOCK_COUNT * 2) as u16 + 2 + 1;
const NEXT_BLOCK_FRAME_HEIGHT: u16 = block::POINT_OF_BLOCK_COUNT as u16 + 1 + 2 + 1;

const RECORD_LEFT_WIDTH: u16 = " time: ".len() as u16;
const RECORD_RIGHT_WIDTH: u16 = 10;
const RECORD_FRAME_WIDTH: u16 = RECORD_LEFT_WIDTH + RECORD_RIGHT_WIDTH + 2;
const RECORD_FRAME_HEIGHT: u16 =
    (std::mem::size_of::<state::Record>() / std::mem::size_of::<u32>()) as u16 + 2 + 2;

const RIGHT_SIDE_WIDTH: u16 = {
    if NEXT_BLOCK_FRAME_WIDTH > RECORD_FRAME_WIDTH {
        NEXT_BLOCK_FRAME_WIDTH
    } else {
        RECORD_FRAME_WIDTH
    }
};

const RECORD_COLOR: Color = Color::Red;
const BOARDER_COLOR: Color = Color::White;

static TERMINAL_WIDTH: AtomicU16 = AtomicU16::new(0);
static TERMINAL_HEIGHT: AtomicU16 = AtomicU16::new(0);

pub trait Frame {
    fn draw(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::draw_border(painter, state);
        Self::draw_inner(painter, state);
    }

    fn get_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16);

    fn get_inner_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16) {
        let (top, bottom, left, right) = Self::get_borders(state);
        (top + 1, bottom - 1, left + 1, right - 1)
    }

    fn draw_border(painter: &Arc<painter::Painter>, state: &Arc<state::State>);
    fn draw_inner(painter: &Arc<painter::Painter>, state: &Arc<state::State>);

    fn flush_terminal_size(painter: &Arc<painter::Painter>) {
        let (col, row) = painter.get_size().unwrap_or((0, 0));
        TERMINAL_WIDTH.store(col, Ordering::Relaxed);
        TERMINAL_HEIGHT.store(row, Ordering::Relaxed);
    }

    fn get_terminal_size() -> (u16, u16) {
        (
            TERMINAL_WIDTH.load(Ordering::Relaxed),
            TERMINAL_HEIGHT.load(Ordering::Relaxed),
        )
    }

    fn set_terminal_size(size: (u16, u16), painter: &Arc<painter::Painter>) {
        painter.resize(size).unwrap();
    }

    fn test_terminal_size(
        state: &Arc<state::State>,
    ) -> Result<((u16, u16), (u16, u16)), ((u16, u16), (u16, u16))> {
        let (col, row) = Self::get_terminal_size();

        let (c, r) = state.get_game_size();
        let (c, r) = (c + 2 + RIGHT_SIDE_WIDTH, r + 2);

        if row >= r && col >= c {
            Ok(((col, c), (row, r)))
        } else {
            Err(((col, c), (row, r)))
        }
    }

    /// (top, bottom, left, right, middle)
    fn get_global_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16, u16) {
        let ((col, c), (row, r)) = Self::test_terminal_size(state).unwrap_or_else(|_| {
            *state.message.lock().unwrap() = Some(String::from("terminal size is too small"));
            panic!("terminal size is too small");
        });

        let top = (row - r) / 2;
        let bottom = top + r - 1;
        let left = (col - c) / 2;
        let right = left + c - 1;
        let middle = right - RIGHT_SIDE_WIDTH;

        (top, bottom, left, right, middle)
    }
}

pub struct GameFrame;
impl GameFrame {
    pub fn draw_pause(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, bottom, left, right) = Self::get_inner_borders(state);
        let (color, s, width, height) = PAUSE_PRINT;
        let pos = (
            (left + right - width) / 2 + 1,
            (bottom + top - height) / 2 + 1,
        );
        let it = s.split('\n').into_iter().map(|s| s.as_bytes());

        painter.clear((top, bottom, left, right)).unwrap();
        painter.multiple_writeln_at(color, pos, it).unwrap();
    }

    pub fn draw_blinking(
        painter: &Arc<painter::Painter>,
        state: &Arc<state::State>,
        lines: &Vec<usize>,
    ) {
        let (_, bottom, left, _) = Self::get_inner_borders(state);
        let block_count = state.get_size().0;

        for i in lines {
            let left_bottom = (left, bottom - *i as u16);
            painter
                .write_at(
                    BOARDER_COLOR,
                    left_bottom,
                    "--".repeat(block_count).as_bytes(),
                )
                .unwrap();
        }
    }

    pub fn draw_falling(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::draw_color_falling(None, painter, state);
    }

    pub fn reset_falling(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::draw_color_falling(Some(Color::Reset), painter, state);
    }

    pub fn draw_color_falling(
        color: Option<Color>,
        painter: &Arc<painter::Painter>,
        state: &Arc<state::State>,
    ) {
        let (_, bottom, left, _) = Self::get_inner_borders(state);
        let (col, row) = state.get_size();

        let left_bottom = (left, bottom);

        let lock = state.two_blocks.lock().unwrap();
        let color = color.unwrap_or(*lock.current_block().color());
        let points = lock.current_block().points();

        // ! danger of dead lock: two_block and painter
        painter
            .draw_multiple_block_at(
                color,
                left_bottom,
                points.iter().filter(|p| {
                    debug_assert!(!(p.x < 0 || p.y < 0 || (p.x as usize) >= col));
                    (p.y as usize) < row
                }),
            )
            .unwrap();
    }

    pub fn draw_stacked(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (_, bottom, left, _) = Self::get_inner_borders(state);
        let left_bottom = (left, bottom);

        // ! danger of dead lock: stacked_blocks and painter
        let lock = state.stacked_blocks.lock().unwrap();
        let it = lock
            .colors
            .iter()
            .enumerate()
            .map(|(y, line)| {
                line.iter().enumerate().filter_map(move |(x, color)| {
                    if let Color::Reset = color {
                        None
                    } else {
                        Some((*color, block::Point::new(x as isize, y as isize)))
                    }
                })
            })
            .flatten();

        painter
            .draw_multiple_color_block_at(left_bottom, it)
            .unwrap();
    }
}

impl Frame for GameFrame {
    fn get_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16) {
        let (top, bottom, left, _, right) = Self::get_global_borders(state);
        (top, bottom, left, right)
    }

    fn draw_border(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, bottom, left, right) = Self::get_borders(state);
        painter
            .draw_rect(BOARDER_COLOR, (top, bottom, left, right))
            .unwrap();
    }

    fn draw_inner(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, bottom, left, right) = Self::get_inner_borders(state);
        painter.clear((top, bottom, left, right)).unwrap();
        Self::draw_stacked(painter, state);
        Self::draw_falling(painter, state);
    }
}

pub struct RecordFrame;
impl RecordFrame {
    pub fn secs_to_string(secs: u32) -> String {
        let s = secs % 60;
        let m = secs / 60 % 60;
        let h = secs / 60 / 60;
        format!("{h:02}:{m:02}:{s:02}")
    }

    pub fn draw_time(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, _, left, _) = Self::get_inner_borders(state);
        let pos = (left + RECORD_LEFT_WIDTH, top + 2);
        let secs = state.record.lock().unwrap().secs;

        painter
            .write_at(
                RECORD_COLOR,
                pos,
                format!(
                    "{:^width$}",
                    Self::secs_to_string(secs),
                    width = RECORD_RIGHT_WIDTH as usize
                )
                .as_bytes(),
            )
            .unwrap();
    }
}

impl Frame for RecordFrame {
    fn get_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16) {
        let (_, bottom, _, _, left) = Self::get_global_borders(state);
        let (top, left) = (bottom - RECORD_FRAME_HEIGHT, left + 1);
        let right = left + RECORD_FRAME_WIDTH;
        (top, bottom, left, right)
    }

    fn draw_border(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, bottom, left, right) = Self::get_borders(state);

        painter
            .draw_rect(BOARDER_COLOR, (top, bottom, left, right))
            .unwrap();

        let (top, _, left, _) = Self::get_inner_borders(state);

        painter
            .write_at(
                BOARDER_COLOR,
                (left, top),
                format!(
                    " {:^width$}",
                    "RECORD",
                    width = RECORD_FRAME_WIDTH as usize - 2
                )
                .as_bytes(),
            )
            .unwrap();
    }

    fn draw_inner(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, _, left, _) = Self::get_inner_borders(state);
        let pos = (left, top + 1);

        let state::Record {
            secs,
            line,
            score,
            speed,
        } = *state.record.lock().unwrap();

        let record_string = format!(
            concat!(
                " \n",
                " time: {:^width$}\n",
                " \n",
                " line: {:^width$}\n",
                " score:{:^width$}\n",
                " speed:{:^width$}"
            ),
            Self::secs_to_string(secs),
            line,
            score,
            speed,
            width = RECORD_RIGHT_WIDTH as usize
        );

        painter
            .multiple_writeln_at(
                RECORD_COLOR,
                pos,
                record_string.split('\n').into_iter().map(|s| s.as_bytes()),
            )
            .unwrap();
    }
}

pub struct NextBlockFrame;

impl NextBlockFrame {
    pub fn draw_next(
        color: Option<Color>,
        painter: &Arc<painter::Painter>,
        state: &Arc<state::State>,
    ) {
        let (_, bottom, left, _) = Self::get_inner_borders(state);
        let left_bottom = (
            left + (NEXT_BLOCK_FRAME_WIDTH - (block::POINT_OF_BLOCK_COUNT as u16 * 2)) / 2,
            bottom + 1 - (NEXT_BLOCK_FRAME_HEIGHT - block::POINT_OF_BLOCK_COUNT as u16) / 2,
        );

        let color = color.unwrap_or(*state.two_blocks.lock().unwrap().next_block().color());
        let points = *state.two_blocks.lock().unwrap().next_block().points();

        painter
            .draw_multiple_block_at(color, left_bottom, points.iter())
            .unwrap();
    }

    pub fn reset_inner(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::draw_next(Some(Color::Reset), painter, state);
    }
}

impl Frame for NextBlockFrame {
    fn get_borders(state: &Arc<state::State>) -> (u16, u16, u16, u16) {
        let (top, _, _, _, left) = Self::get_global_borders(state);
        let (bottom, left) = (top + NEXT_BLOCK_FRAME_HEIGHT - 1, left + 1);
        let right = left + NEXT_BLOCK_FRAME_WIDTH;
        (top, bottom, left, right)
    }

    fn draw_border(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let (top, bottom, left, right) = Self::get_borders(state);

        painter
            .draw_rect(BOARDER_COLOR, (top, bottom, left, right))
            .unwrap();

        let (top, _, left, _) = Self::get_inner_borders(state);

        painter
            .write_at(
                BOARDER_COLOR,
                (left, top),
                format!(
                    " {:^width$}",
                    "NEXT",
                    width = NEXT_BLOCK_FRAME_WIDTH as usize - 2
                )
                .as_bytes(),
            )
            .unwrap();
    }

    fn draw_inner(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::draw_next(None, painter, state);
    }
}
