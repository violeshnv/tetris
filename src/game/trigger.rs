use super::painter;
use super::state;

use std::sync::{
    mpsc::{sync_channel, Receiver, SyncSender},
    Arc,
};

use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use terminal::{Event, KeyCode, KeyEvent, KeyModifiers};

const INITIAL_DROP_INTERVAL_SECOND_COUNT: u64 = 2;

fn speed_to_duration(speed: u32) -> Duration {
    Duration::from_secs(INITIAL_DROP_INTERVAL_SECOND_COUNT) / speed
}

#[derive(Default)]
pub struct Trigger {
    pub threads: Vec<JoinHandle<()>>,
}

impl Drop for Trigger {
    fn drop(&mut self) {
        while let Some(t) = self.threads.pop() {
            t.join().unwrap();
        }
    }
}

impl Trigger {
    pub fn start(
        &mut self,
        painter: Arc<painter::Painter>,
        state: Arc<state::State>,
    ) -> (Receiver<u64>, Receiver<Event>) {
        let (_, timer_rx) = self.timer_thread(state.clone());
        let (event_tx, event_rx) = self.event_thread(painter, state.clone());
        self.auto_drop_thread(event_tx, state.clone());

        (timer_rx, event_rx)
    }

    fn timer_thread(&mut self, state: Arc<state::State>) -> (SyncSender<u64>, Receiver<u64>) {
        let mut duration = Duration::from_secs(1);

        let (tx, rx) = sync_channel(0);
        let tx_clone = tx.clone();

        let handle = thread::spawn(move || loop {
            let (lock, cond) = state.timer.lock_cond();
            let mut keeper = lock.lock().unwrap();

            while keeper.is_paused() {
                keeper = cond.wait(keeper).unwrap();
            }

            if let Some(schedule) = keeper.scheduled_time::<0>(duration) {
                drop(keeper);

                let diff = schedule.duration_since(Instant::now());
                if diff.is_zero() {
                    if tx.send(duration.as_secs()).is_err() {
                        break;
                    }
                    duration += Duration::from_secs(1);
                } else {
                    thread::sleep(diff);
                }
            } else {
                panic!("keeper ensure no pause");
            }
        });

        self.threads.push(handle);
        (tx_clone, rx)
    }

    fn event_thread(
        &mut self,
        painter: Arc<painter::Painter>,
        _: Arc<state::State>,
    ) -> (SyncSender<Event>, Receiver<Event>) {
        let (tx, rx) = sync_channel(1);
        let tx_clone = tx.clone();

        let handle = thread::spawn(move || loop {
            if let Some(event) = painter.get_event().unwrap_or(None) {
                if tx.send(event).is_err() {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(20));
        });

        self.threads.push(handle);
        (tx_clone, rx)
    }

    fn auto_drop_thread(&mut self, keyboard_sender: SyncSender<Event>, state: Arc<state::State>) {
        let handler = thread::spawn(move || loop {
            let speed = state.record.lock().unwrap().speed;
            let duration = speed_to_duration(speed);

            let (lock, cond) = state.timer.lock_cond();
            let mut keeper = lock.lock().unwrap();

            while keeper.is_paused() {
                keeper = cond.wait(keeper).unwrap();
            }

            if let Some(schedule) = keeper.scheduled_time::<1>(duration) {
                drop(keeper);

                let diff = schedule.duration_since(Instant::now());
                if diff.is_zero() {
                    if keyboard_sender
                        .send(Event::Key(KeyEvent {
                            code: KeyCode::Down,
                            modifiers: KeyModifiers::empty(),
                        }))
                        .is_err()
                    {
                        break;
                    }
                    thread::sleep(duration / 2);
                } else {
                    thread::sleep(diff);
                }
            } else {
                panic!("keeper ensure no pause");
            }
        });

        self.threads.push(handler);
    }
}
