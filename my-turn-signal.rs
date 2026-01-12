#![no_main]
#![no_std]

use cortex_m_rt::entry;
use embedded_hal::digital::InputPin;
use microbit::hal::timer::Timer;
use microbit::{display::blocking::Display, Board};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

enum Light {
    Lit,
    Unlit,
}

struct Duration {
    max: u16,
    cur: u16,
}

struct State {
    light: Light,
    signal: SignalType,
    lit_duration: Duration,
    unlit_duration: Duration,
}

#[derive(PartialEq, Eq)]
enum SignalType {
    Left,
    Right,
    Straight,
}

impl State {
    fn new(lit_duration: u16, unlit_duration: u16) -> Self {
        State {
            light: Light::Lit,
            signal: SignalType::Straight,
            lit_duration: Duration {
                max: lit_duration,
                cur: lit_duration,
            },
            unlit_duration: Duration {
                max: unlit_duration,
                cur: unlit_duration,
            },
        }
    }

    fn tick(&mut self) -> [[u8; 5]; 5] {
        match self.signal {
            SignalType::Left | SignalType::Right => match self.light {
                Light::Lit => {
                    self.lit_duration.cur -= 1;
                    if self.lit_duration.cur == 0 {
                        self.light = Light::Unlit;
                        self.unlit_duration.cur = self.unlit_duration.max;
                    }
                }
                Light::Unlit => {
                    self.unlit_duration.cur -= 1;
                    if self.unlit_duration.cur == 0 {
                        self.light = Light::Lit;
                        self.lit_duration.cur = self.lit_duration.max;
                    }
                }
            },
            _ => {}
        }

        match self.light {
            Light::Unlit => [
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
                [0, 0, 0, 0, 0],
            ],
            Light::Lit => match self.signal {
                SignalType::Left => [
                    [0, 0, 1, 0, 0],
                    [0, 1, 0, 0, 0],
                    [1, 1, 1, 1, 1],
                    [0, 1, 0, 0, 0],
                    [0, 0, 1, 0, 0],
                ],
                SignalType::Right => [
                    [0, 0, 1, 0, 0],
                    [0, 0, 0, 1, 0],
                    [1, 1, 1, 1, 1],
                    [0, 0, 0, 1, 0],
                    [0, 0, 1, 0, 0],
                ],
                SignalType::Straight => [
                    [0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0],
                    [0, 0, 1, 0, 0],
                    [0, 0, 0, 0, 0],
                    [0, 0, 0, 0, 0],
                ],
            },
        }
    }

    fn set_signal(&mut self, state: SignalType) {
        if state != self.signal {
            //reset counters
            self.unlit_duration.cur = self.unlit_duration.max;
            self.lit_duration.cur = self.lit_duration.max;
            self.light = Light::Lit;
        }

        self.signal = state;
    }
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let mut display = Display::new(board.display_pins);

    // Configure buttons
    let mut button_a = board.buttons.button_a;
    let mut button_b = board.buttons.button_b;

    let mut state = State::new(25, 55);

    loop {
        let a_pressed = button_a.is_low().unwrap();
        let b_pressed = button_b.is_low().unwrap();

        if a_pressed {
            state.set_signal(SignalType::Left);
        } else if b_pressed {
            state.set_signal(SignalType::Right)
        } else {
            state.set_signal(SignalType::Straight)
        }

        display.show(&mut timer, state.tick(), 10);
    }
}
