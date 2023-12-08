use terminal::{
    self, Event as Tevent, KeyCode as Code, KeyEvent as Kevent,
    KeyModifiers::{self as Modifiers},
};

const EMPTY: Modifiers = Modifiers::empty();
// const SHIFT: Modifiers = Modifiers::SHIFT;
const CONTROL: Modifiers = Modifiers::CONTROL;
// const ALT: Modifiers = Modifiers::ALT;

pub enum Event {
    Unknow,
    Quit,
    Toggle, // Pause or Resume
    ClockRotate,
    InverseRotate,
    Drop,
    Left,
    Right,
    HardDrop,
    Resize,
}

impl From<Tevent> for Event {
    fn from(value: terminal::Event) -> Self {
        match value {
            Tevent::Key(key) => {
                let Kevent {
                    code: c,
                    modifiers: m,
                } = key;

                match (c, m) {
                    (Code::Char('c'), CONTROL) | (Code::Char('q'), EMPTY) => Event::Quit,
                    (Code::Char('p'), EMPTY) => Event::Toggle,
                    (Code::Up | Code::Char('z'), EMPTY) => Event::ClockRotate,
                    (Code::Char('x'), EMPTY) => Event::InverseRotate,
                    (Code::Down, EMPTY) => Event::Drop,
                    (Code::Left, EMPTY) => Event::Left,
                    (Code::Right, EMPTY) => Event::Right,
                    (Code::Char(' '), EMPTY) => Event::HardDrop,
                    _ => Event::Unknow,
                }
            }
            Tevent::Resize => Event::Resize,
            _ => Event::Unknow,
        }
    }
}
