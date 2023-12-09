use std::sync::Arc;

use self::frame::Frame;

mod block;
mod event;
mod frame;
mod handler;
mod painter;
mod state;
mod timer;
mod trigger;

pub struct Game {
    // definition order is important to drop order
    // keyboard event -> handler (reciever) drop
    //                -> trigger (sender) drop
    //                -> painter drop (exit alternative screen)
    //                -> state drop (print settlement)
    handler: handler::Handler,
    trigger: trigger::Trigger,
    painter: Arc<painter::Painter>,
    state: Arc<state::State>,
}

impl Game {
    pub fn new() -> Self {
        Game {
            handler: Default::default(),
            trigger: Default::default(),
            painter: Arc::new(painter::Painter::new()),
            state: Arc::new(state::State::new(
                10,
                20,
                Default::default(),
                Default::default(),
            )),
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if let Err(((col, c), (row, r))) = {
            frame::GameFrame::flush_terminal_size(&self.painter);
            frame::GameFrame::test_terminal_size(&self.state)
        } {
            return Err(format!(
                concat!(
                    "terminal screen (column x row) too small: ",
                    "current screen is {}x{} , ",
                    "need at least {}x{}"
                ),
                col, row, c, r
            ));
        } else {
            let (timer_rx, event_rx) = self.trigger.start(self.painter.clone(), self.state.clone());
            self.handler
                .start(self.painter.clone(), self.state.clone(), timer_rx, event_rx);

            self.state.timer.resume();
            Ok(())
        }
    }
}
