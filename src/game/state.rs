use super::block;
use super::timer;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Debug)]
pub struct Record {
    pub secs: u32, // seconds
    pub line: u32,
    pub score: u32,
    pub speed: u32,
}

impl Default for Record {
    fn default() -> Self {
        Record {
            secs: 0,
            line: 0,
            score: 0,
            speed: 1,
        }
    }
}

pub struct TwoBlocks {
    curr: Box<block::FallingBlock>,
    next: Box<block::FallingBlock>,
}

impl TwoBlocks {
    pub fn new(curr: Box<block::FallingBlock>, next: Box<block::FallingBlock>) -> Self {
        TwoBlocks { curr, next }
    }

    pub fn current_block(&self) -> &block::FallingBlock {
        &*self.curr
    }

    pub fn current_block_mut(&mut self) -> &mut block::FallingBlock {
        &mut *self.curr
    }

    pub fn next_block(&self) -> &block::FallingBlock {
        &*self.next
    }

    pub fn push(&mut self, mut block: Box<block::FallingBlock>) -> Box<block::FallingBlock> {
        use std::mem::swap;
        swap(&mut self.curr, &mut self.next);
        swap(&mut self.next, &mut block);
        block
    }
}

pub struct State {
    size: (usize, usize),

    pub quit_signal: AtomicBool,
    pub handle_signal: AtomicBool,

    pub timer: timer::Timer,
    pub record: Mutex<Record>,

    pub two_blocks: Mutex<TwoBlocks>,

    pub stacked_blocks: Mutex<block::StackedBlock>,

    pub message: Mutex<Option<String>>,
}

impl State {
    pub fn new(
        column: usize,
        row: usize,
        curr: block::FallingBlock,
        next: block::FallingBlock,
    ) -> Self {
        State {
            size: (column, row),
            quit_signal: AtomicBool::new(false),
            handle_signal: AtomicBool::new(true),
            timer: Default::default(),
            record: Default::default(),
            two_blocks: Mutex::new(TwoBlocks::new(Box::new(curr), Box::new(next))),
            stacked_blocks: Mutex::new(block::StackedBlock::new(column, row)),
            message: Default::default(),
        }
    }

    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn get_game_size(&self) -> (u16, u16) {
        let (col, row) = self.get_size();
        (2 * col as u16, row as u16)
    }

    pub fn quit(&self) -> bool {
        self.quit_signal.load(Ordering::Relaxed)
    }
}

impl Drop for State {
    fn drop(&mut self) {
        self.quit_signal.store(true, Ordering::Relaxed);

        let Record {
            secs,
            line,
            score,
            speed,
        } = *self.record.lock().unwrap();

        if let Some(message) = &*self.message.lock().unwrap() {
            println!("{}", message);
        }

        if score == 0 {
            return;
        }

        println!(
            concat!(
                "😃 you got score: {}, ",
                "eliminated {} line(s), ",
                "reached a speed of {}, ",
                "and played for {} second(s) at this game"
            ),
            score, line, speed, secs
        );
    }
}
