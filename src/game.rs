use std::sync::Arc;

mod block;
mod event;
mod frame;
mod handler;
mod painter;
mod random;
mod state;
mod timer;
mod trigger;

pub struct Game {
    // definition order is important to drop order
    handler: handler::Handler,
    trigger: trigger::Trigger,
    painter: Arc<painter::Painter>,
    state: Arc<state::State>,
}

impl Game {
    pub fn new() -> Self {
        Game {
            painter: Arc::new(painter::Painter::new()),
            state: Arc::new(state::State::new(
                10,
                20,
                Default::default(),
                Default::default(),
            )),
            trigger: Default::default(),
            handler: Default::default(),
        }
    }

    pub fn start(&mut self) {
        let (timer_rx, event_rx) = self.trigger.start(self.painter.clone(), self.state.clone());
        self.handler
            .start(self.painter.clone(), self.state.clone(), timer_rx, event_rx);

        self.state.timer.resume();
    }
}
