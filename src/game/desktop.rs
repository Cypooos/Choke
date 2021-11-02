use lazy_static::lazy_static;

use core::{fmt, usize};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

#[cfg(feature = "audio")]
use crate::game::audio;
use crate::{
    game::screens::{
        screens::{screen_to_instance, Screen},
        Screenable, SA,
    },
    io::vga_writer,
};
use alloc::{boxed::Box, vec::Vec};
use core::mem;

lazy_static! {
    static ref DESKTOP_LOGGER: [[vga_writer::ScreenChar; vga_writer::BUFFER_WIDTH]; vga_writer::BUFFER_HEIGHT] =
        [[vga_writer::DEFAULT_SCREENCHAR; vga_writer::BUFFER_WIDTH]; vga_writer::BUFFER_HEIGHT];
}

lazy_static! {
    pub static ref DESKTOP: Mutex<DesktopTUI> = Mutex::new(DesktopTUI {
        _mouse_pos: (5, 5),
        active_screen: screen_to_instance(Screen::MainMenu),
        paused_screens: Vec::new(),
        time: 0,
    });
}

pub struct DesktopTUI {
    _mouse_pos: (usize, usize),
    active_screen: Box<dyn Screenable>,
    paused_screens: Vec<Box<dyn Screenable>>,
    // levels: HashMap<String, Box<dyn Screenable>>,
    time: u16,
}

impl fmt::Write for DesktopTUI {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        vga_writer::WRITER.lock().write_string(s);
        Ok(())
    }
}

impl DesktopTUI {
    pub fn start(&mut self) {
        if let Some(x) = self.active_screen.init() {
            self.execute_actions(x)
        }
    }

    pub fn int_time(&mut self) {
        self.time = self.time.checked_add(1).unwrap_or(0);
        #[cfg(feature = "audio")]
        {
            audio::AUDIOMANAGER.lock().frame();
        }
        if (self.time & 0xFF) == 0 {
            if let Some(x) = self.active_screen.on_time((self.time >> 8) as u8) {
                self.execute_actions(x);
            }
            #[cfg(feature = "info-bar")]
            {
                vga_write!(
                    0,
                    0,
                    "$3F                                                                    <Discursif/>"
                );
                vga_write!(
                    0,
                    0,
                    "Choke vb{} \u{B3} $!5{:?}$!F \u{B3} $!5{:?}$!F \u{B3} \x01",
                    env!("CARGO_PKG_VERSION"),
                    self.paused_screens.len(),
                    self.time >> 8
                );
                // vga_write!(0, 0, "111111111{}2222", "");
                // qemu_debug!("111111111{}2222", "");
            }
        }
    }

    fn draw(&mut self) {
        for mut x in &self.paused_screens {
            x.draw();
            #[cfg(feature = "info-bar")]
            {
                vga_write!(
                    0,
                    0,
                    "$3F                                                                    <Discursif/>"
                );
                vga_write!(
                    0,
                    0,
                    "Choke vb{}\u{B3} $!5{:?}$!F \u{B3} $!5{:?}$!F \u{B3} \x01",
                    env!("CARGO_PKG_VERSION"),
                    self.paused_screens.len(),
                    self.time
                );
            }
        }
        self.active_screen.draw();
        return;
    }

    pub fn execute_actions(&mut self, actions: Vec<SA>) {
        for action in actions {
            match action {
                SA::Change(x) => {
                    self.active_screen = screen_to_instance(x);
                    self.active_screen.init();
                }
                SA::Overwrite(x) => {
                    self.active_screen = screen_to_instance(x);
                    self.active_screen.init();
                    self.paused_screens = Vec::new();
                }
                SA::Load(x) => {
                    let old = mem::replace(&mut self.active_screen, screen_to_instance(x));
                    self.paused_screens.push(old);
                }
                SA::Restore => {
                    self.active_screen = self
                        .paused_screens
                        .pop()
                        .unwrap_or(screen_to_instance(Screen::MainMenu));
                    self.active_screen.init();
                }
                SA::Draw => self.draw(),
            }
        }
    }

    pub fn int_key(&mut self, scancode: u8) {
        lazy_static! {
            static ref KEYBOARD: Mutex<Keyboard<layouts::Azerty, ScancodeSet1>> = Mutex::new(
                Keyboard::new(layouts::Azerty, ScancodeSet1, HandleControl::Ignore)
            );
        };

        let mut keyboard = KEYBOARD.lock();

        // Detect key
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            qemu_debug!("key:{:?}", key_event);

            if let Some(x) = self.active_screen.on_key(
                key_event.clone(),
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(character) => Some(character),
                        DecodedKey::RawKey(_) => None,
                    }
                } else {
                    None
                },
            ) {
                self.execute_actions(x)
            }
        } else {
            qemu_debug!("Unknow keyboard interrupt");
        };
        self.draw();
        //vga_print!("desktop:{},", scancode);
    }
}

#[doc(hidden)]
pub fn _print(_args: fmt::Arguments) {
    // interrupts::without_interrupts(|| {
    //     qemu_print!("ohdqiujhfs");
    //     DESKTOP.lock().write_fmt(args).unwrap();
    // });
}
