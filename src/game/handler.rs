use super::frame::Frame;
use super::state;
use super::{block, event, frame, painter};

use std::borrow::BorrowMut;
use std::ops::AddAssign;
use std::sync::atomic::Ordering;
use std::sync::{mpsc::Receiver, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use terminal::Event;

const TO_LEFT_POINT: block::Point = block::Point::new(-1, 0);
const TO_RIGHT_POINT: block::Point = block::Point::new(1, 0);
const TO_DROP_POINT: block::Point = block::Point::new(0, -1);
const TO_RISE_POINT: block::Point = block::Point::new(0, 1);

fn line_to_score(line: u32) -> u32 {
    (1..=line).into_iter().sum()
}

#[derive(Default)]
pub struct Handler {
    pub threads: Vec<JoinHandle<()>>,
}

impl Drop for Handler {
    fn drop(&mut self) {
        while let Some(t) = self.threads.pop() {
            t.join().unwrap();
        }
    }
}

impl Handler {
    pub fn is_valid_position(points: &[block::Point], state: &Arc<state::State>) -> bool {
        let (col, row) = state.get_size();
        let (col, _) = (col as isize, row as isize);

        points
            .iter()
            .all(|p| 0 <= p.x && p.x < col && 0 <= p.y ) // p.y < row
            && !state.stacked_blocks.lock().unwrap().is_overlapped(points)
    }

    fn resize(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        state.timer.pause();
        state.handle_signal.store(false, Ordering::Relaxed);

        let size = frame::GameFrame::get_terminal_size();
        frame::GameFrame::flush_terminal_size(painter);

        if frame::GameFrame::test_terminal_size(state).is_err() {
            // try to recover
            frame::GameFrame::set_terminal_size(size, painter);

            frame::GameFrame::flush_terminal_size(painter);
            if frame::GameFrame::test_terminal_size(state).is_err() {
                // fail to recover
                Self::quit(painter, state);
            }
        } else {
            painter.clear_all().unwrap();
            frame::GameFrame::draw(painter, state);
            frame::RecordFrame::draw(painter, state);
            frame::NextBlockFrame::draw(painter, state);
        }

        state.handle_signal.store(true, Ordering::Release);
        state.timer.resume();
    }

    fn quit(_: &Arc<painter::Painter>, state: &Arc<state::State>) {
        state.quit_signal.store(true, Ordering::Relaxed);
    }

    fn pause(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        state.timer.pause();
        frame::GameFrame::draw_pause(painter, state);
    }

    fn resume(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        frame::GameFrame::draw_inner(painter, state);
        state.timer.resume();
    }

    fn drop(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        // ! danger of dead lock: two_blocks, stacked_blocks
        // ! drop to avoid dead lock
        frame::GameFrame::reset_falling(painter, state);

        let mut lock = state.two_blocks.lock().unwrap();

        let block = lock.current_block_mut();
        block.shift(&TO_DROP_POINT);

        if Self::is_valid_position(block.points(), state) {
            // likely
            drop(lock);
            frame::GameFrame::draw_falling(painter, state);
        } else {
            // unlikely
            block.shift(&TO_RISE_POINT);
            let color = block.color();

            state
                .stacked_blocks
                .lock()
                .unwrap()
                .borrow_mut()
                .cover(*color, block.points());

            drop(lock);
            frame::GameFrame::draw_stacked(painter, state);
            Self::generate_new_block(painter, state);
            Self::settle_up(painter, state);
        }

        state.timer.lock_cond().0.lock().unwrap().set_now::<1>();
    }

    fn translate(to: &block::Point, painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let mut points = *state.two_blocks.lock().unwrap().current_block().points();
        points.add_assign(to);

        if Self::is_valid_position(&points, state) {
            // in frame
            frame::GameFrame::reset_falling(painter, state);

            state
                .two_blocks
                .lock()
                .unwrap()
                .current_block_mut()
                .shift(to);

            frame::GameFrame::draw_falling(painter, state);
        }
    }

    fn left(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::translate(&TO_LEFT_POINT, painter, state);
    }

    fn right(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::translate(&TO_RIGHT_POINT, painter, state);
    }

    fn rotate(direction: isize, painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        frame::GameFrame::reset_falling(painter, state);
        {
            let mut lock = state.two_blocks.lock().unwrap();
            let block = lock.current_block_mut();

            *block += direction;
            if !Self::is_valid_position(block.points(), state) {
                *block += -direction;
            }
        }
        frame::GameFrame::draw_falling(painter, state);
    }

    fn clock_rotate(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::rotate(1, painter, state);
    }

    fn inverse_rotate(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        Self::rotate(-1, painter, state);
    }

    fn hard_drop(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        // ! danger of dead lock: two_blocks, stacked_blocks
        // ! drop to avoid dead lock
        frame::GameFrame::reset_falling(painter, state);

        let mut lock = state.two_blocks.lock().unwrap();
        let block = lock.current_block_mut();

        while {
            block.shift(&TO_DROP_POINT);
            Self::is_valid_position(block.points(), state)
        } {}
        block.shift(&TO_RISE_POINT);
        drop(lock);

        Self::drop(painter, state);
    }

    fn score_update(
        line: u32,
        score: u32,
        painter: &Arc<painter::Painter>,
        state: &Arc<state::State>,
    ) {
        // ! drop to avoid dead lock
        let mut lock = state.record.lock().unwrap();
        lock.line += line;
        lock.score += score;
        drop(lock);

        frame::RecordFrame::draw_inner(painter, state);
    }

    fn time_update(secs: u32, painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        state.record.lock().unwrap().secs = secs;
        frame::RecordFrame::draw_time(painter, state);
    }

    // fn speed_update(speed: u32, painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
    //     state.record.lock().unwrap().speed = speed;
    //     frame::RecordFrame::draw_inner(painter, state);
    // }

    fn settle_up(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        let full_lines = state.stacked_blocks.lock().unwrap().full_lines();
        if !full_lines.is_empty() {
            Self::blink(&full_lines, Duration::from_millis(400), 2, painter, state);

            let line = full_lines.len() as u32;
            state.stacked_blocks.lock().unwrap().eliminate(&full_lines);
            Self::score_update(line, line_to_score(line), painter, state);
        }
        frame::GameFrame::draw_inner(painter, state);
    }

    fn blink(
        lines: &Vec<usize>,
        duration: Duration,
        times: i32,
        painter: &Arc<painter::Painter>,
        state: &Arc<state::State>,
    ) {
        state.timer.pause();
        state.handle_signal.store(false, Ordering::Relaxed);

        for _ in 0..times {
            frame::GameFrame::draw_blinking(painter, state, lines);
            painter.flush().unwrap();
            thread::sleep(duration);
            frame::GameFrame::draw_stacked(painter, state);
            painter.flush().unwrap();
            thread::sleep(duration);
        }

        state.handle_signal.store(true, Ordering::Release);
        state.timer.resume();
    }

    fn generate_new_block(painter: &Arc<painter::Painter>, state: &Arc<state::State>) {
        frame::NextBlockFrame::reset_inner(painter, state);

        let x = rand::random::<usize>() % (block::BLOCKS.len() * block::ORIENTATION_COUNT);
        let (b, o) = (x / block::ORIENTATION_COUNT, x % block::ORIENTATION_COUNT);
        let block = Box::new(block::FallingBlock::new(b, o));
        let _block = state.two_blocks.lock().unwrap().push(block);

        let (col, row) = state.get_size();
        let origin = block::Point::new(
            (col - block::POINT_OF_BLOCK_COUNT) as isize / 2,
            row as isize - 1,
        );

        let mut lock = state.two_blocks.lock().unwrap();
        let block = lock.current_block_mut();
        block.shift(&origin);

        if {
            let valid = Self::is_valid_position(block.points(), state);
            drop(lock);
            !valid
        } {
            *state.message.lock().unwrap() = Some(String::from("block stack overflow"));
            Self::quit(painter, state);
        }

        frame::GameFrame::draw_falling(painter, state);
        frame::NextBlockFrame::draw_inner(painter, state);
    }
}

impl Handler {
    pub fn start(
        &mut self,
        painter: Arc<painter::Painter>,
        state: Arc<state::State>,
        timer_rx: Receiver<u64>,
        event_rx: Receiver<Event>,
    ) {
        self.time_update_thread(timer_rx, painter.clone(), state.clone());
        self.event_thread(event_rx, painter.clone(), state.clone());
    }

    fn time_update_thread(
        &mut self,
        timer_rx: Receiver<u64>,
        painter: Arc<painter::Painter>,
        state: Arc<state::State>,
    ) {
        let handler = thread::spawn(move || {
            while !state.quit() {
                let secs = timer_rx.recv().unwrap() as u32;
                Self::time_update(secs, &painter, &state);
                painter.flush().unwrap();
            }
        });

        self.threads.push(handler);
    }

    fn event_thread(
        &mut self,
        event_rx: Receiver<Event>,
        painter: Arc<painter::Painter>,
        state: Arc<state::State>,
    ) {
        let handler = thread::spawn(move || {
            Self::resize(&painter, &state);
            Self::generate_new_block(&painter, &state);
            frame::GameFrame::draw_inner(&painter, &state);

            painter.flush().unwrap();

            while !state.quit() {
                let event = event_rx.recv().unwrap();
                if !state.handle_signal.load(Ordering::Relaxed) {
                    continue;
                }

                if state.timer.is_paused() {
                    match event::Event::from(event) {
                        event::Event::Toggle => Self::resume(&painter, &state),
                        event::Event::Quit => Self::quit(&painter, &state),
                        event::Event::Resize => Self::resize(&painter, &state),
                        _ => {}
                    }
                    painter.flush().unwrap();

                    continue;
                }

                match event::Event::from(event) {
                    event::Event::Unknow => {}
                    event::Event::Quit => Self::quit(&painter, &state),
                    event::Event::Toggle => Self::pause(&painter, &state),
                    event::Event::ClockRotate => Self::clock_rotate(&painter, &state),
                    event::Event::InverseRotate => Self::inverse_rotate(&painter, &state),
                    event::Event::Drop => Self::drop(&painter, &state),
                    event::Event::Left => Self::left(&painter, &state),
                    event::Event::Right => Self::right(&painter, &state),
                    event::Event::HardDrop => Self::hard_drop(&painter, &state),
                    event::Event::Resize => Self::resize(&painter, &state),
                }

                painter.flush().unwrap();
            }
        });

        self.threads.push(handler);
    }
}
