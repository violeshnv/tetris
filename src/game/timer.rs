use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

pub struct TimeKeeper<const N: usize> {
    times: [Instant; N],
    pause_time: Instant,
    pause: bool,
}

impl<const N: usize> Default for TimeKeeper<N> {
    fn default() -> Self {
        TimeKeeper {
            times: [Instant::now(); N],
            pause_time: Instant::now(),
            pause: true,
        }
    }
}

impl<const N: usize> TimeKeeper<N> {
    pub fn new(time: Instant) -> Self {
        TimeKeeper {
            times: [time; N],
            ..Default::default()
        }
    }

    // pub fn duration_since<const M: usize>(&self, earlier: Instant) -> Duration {
    //     self.times[M].duration_since(earlier)
    // }

    pub fn scheduled_time<const M: usize>(&self, duration: Duration) -> Option<Instant> {
        if self.pause {
            eprintln!("get scheduled time when pausing");
            None
        } else {
            Some(self.times[M] + duration)
        }
    }

    pub fn set_now<const M: usize>(&mut self) {
        self.times[M] = Instant::now();
    }

    // fn toggle(&mut self) -> Option<Duration> {
    //     if self.is_paused() {
    //         Some(self.resume())
    //     } else {
    //         self.pause();
    //         None
    //     }
    // }

    fn pause(&mut self) {
        if self.is_paused() {
            eprintln!("pause paused timer again!");
            return;
        }
        self.pause_time = Instant::now();
        self.pause = true;
    }

    fn resume(&mut self) -> Duration {
        if self.is_paused() {
            let duration = self.pause_time.elapsed();
            for time in &mut self.times {
                *time += duration;
            }
            self.pause = false;
            duration
        } else {
            eprintln!("resume running timer again!");
            Duration::from_secs(0)
        }
    }

    pub fn is_paused(&self) -> bool {
        self.pause
    }
}

pub struct Timer {
    keeper: Arc<(Mutex<TimeKeeper<2>>, Condvar)>,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new(Instant::now())
    }
}

impl Timer {
    pub fn new(time: Instant) -> Self {
        Timer {
            keeper: Arc::new((Mutex::new(TimeKeeper::new(time)), Condvar::new())),
        }
    }

    pub fn lock_cond(&self) -> &(Mutex<TimeKeeper<2>>, Condvar) {
        &*self.keeper
    }

    // pub fn toggle(&self) -> Option<Duration> {
    //     let (lock, cond) = self.lock_cond();
    //     if let Some(duration) = lock.lock().unwrap().toggle() {
    //         cond.notify_all();
    //         Some(duration)
    //     } else {
    //         None
    //     }
    // }

    pub fn pause(&self) {
        let (lock, _) = self.lock_cond();
        lock.lock().unwrap().pause();
    }

    pub fn resume(&self) -> Duration {
        let (lock, cond) = self.lock_cond();

        let mut keeper = lock.lock().unwrap();
        let duration = keeper.resume();
        cond.notify_all();

        duration
    }

    pub fn is_paused(&self) -> bool {
        self.lock_cond().0.lock().unwrap().is_paused()
    }
}
