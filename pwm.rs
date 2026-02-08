#![no_main]
#![no_std]

use cortex_m::asm;
use cortex_m_rt::entry;
use critical_section_lock_mut::LockMut;
use embedded_hal::{delay::DelayNs, digital::OutputPin};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

use core::sync::atomic::{
    AtomicI32,
    Ordering::{Acquire, Release},
};

use microbit::{
    hal::{
        gpio,
        pac::{self, interrupt},
        prelude::*,
        pwm,
        spi::Frequency,
        timer,
    },
    Board,
};

/// Base siren frequency in Hz.
const BASE_FREQ: i32 = 440;
const MAX_FREQ: i32 = 660;

/// Max rise in siren frequency in Hz.
const FREQ_RISE: i32 = 10;
/// Time for one full cycle in µs.
const NUMBER_STEPS: i32 = (660 - 440) / FREQ_RISE;
const DURATION_PER_NOTE_MS: i32 = 1000 / (NUMBER_STEPS * 2); //up and down
const CLOCK_CYCLES_PER_NOTE: u32 = DURATION_PER_NOTE_MS as u32 * 1000;

static PWM: LockMut<pwm::Pwm<pac::PWM0>> = LockMut::new();
static TIMER: LockMut<microbit::hal::Timer<pac::TIMER0>> = LockMut::new();
static CUR_FREQ: AtomicI32 = AtomicI32::new(BASE_FREQ);
static DIRECTION: AtomicI32 = AtomicI32::new(1);
/// The timer interrupt for the siren. Just steps the siren.
#[interrupt]
fn TIMER0() {
    PWM.with_lock(|pwm| {
        let scalar = DIRECTION.load(Acquire);
        let freq = CUR_FREQ.fetch_add(FREQ_RISE * scalar, Acquire);
        rprintln!("freq {}", freq);
        if freq == MAX_FREQ - FREQ_RISE && scalar > 0 {
            //reset
            DIRECTION.store(-scalar, Release);
        } else if freq == BASE_FREQ + FREQ_RISE && scalar < 0 {
            DIRECTION.store(-scalar, Release);
        }

        pwm.set_period((freq as u32).hz());
        pwm.set_duty_on_common(500); //required to reset duty cycle after changing freq (to ensure it stays at 50% relative to new configuration)
    });

    TIMER.with_lock(|timer| {
        timer.reset_event();
        timer.start(CLOCK_CYCLES_PER_NOTE);
    });
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let pwm = pwm::Pwm::new(board.PWM0);
    //

    // It is convenient to use a `degrade()`ed pin
    // to avoid having to deal with the type of the
    // speaker pin, rather than looking it up:
    // the pin is stored globally in `SIREN`, so its
    // size must be known.
    //
    // This does lose type safety, but that is unlikely
    // to matter after this point.
    let speaker_pin = board
        .speaker_pin
        .into_push_pull_output(gpio::Level::Low)
        .degrade();

    let mut timer0 = timer::Timer::new(board.TIMER0);
    let mut timer1 = timer::Timer::new(board.TIMER1);

    // Set up the NVIC to handle interrupts.
    unsafe { pac::NVIC::unmask(pac::Interrupt::TIMER0) };
    pac::NVIC::unpend(pac::Interrupt::TIMER0);

    timer0.enable_interrupt();
    timer0.reset_event();
    timer0.start(CLOCK_CYCLES_PER_NOTE);
    TIMER.init(timer0);

    pwm.set_output_pin(pwm::Channel::C0, speaker_pin);
    pwm.set_prescaler(pwm::Prescaler::Div16);
    pwm.set_period((BASE_FREQ as u32).hz()); //Hz
                                             // Start the siren and do the countdown.
    pwm.set_max_duty(1000);
    pwm.set_duty_on_common(500);

    pwm.enable();
    PWM.init(pwm);

    for t in (1..=10).rev() {
        rprintln!("{}", t);
        timer1.delay_ms(1_000);
    }
    rprintln!("launch!");

    PWM.with_lock(|pwm| {
        pwm.disable();
    });

    TIMER.with_lock(|timer| {
        timer.reset_event();
        timer.disable_interrupt();
    });

    loop {
        asm::wfi();
    }
}
