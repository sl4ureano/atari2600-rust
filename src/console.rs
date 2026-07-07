use anyhow::Result;
use crate::{bus::Bus, cartridge::Cartridge, cpu::Cpu6502, input::Input, audio::{SharedAudioState, shared_audio_state}};

pub struct Atari2600 { pub cpu: Cpu6502, pub bus: Bus, pub input: Input, pub trace: bool, pub audio_state: SharedAudioState }

impl Atari2600 {
    pub fn new(rom: Vec<u8>) -> Result<Self> {
        let cart = Cartridge::new(rom)?;
        log::info!("ROM: {} bytes | mapper: {:?} | bank inicial: {}", cart.len(), cart.mapper(), cart.bank());
        let mut bus = Bus::new(cart);
        let mut cpu = Cpu6502::new();
        cpu.reset(&mut bus);
        log::info!("RESET VECTOR => PC=${:04X}", cpu.pc);
        Ok(Self { cpu, bus, input: Input::new(), trace: false, audio_state: shared_audio_state() })
    }
    pub fn set_video_calibration(&mut self, visible_start_clock: usize, x_adjust: isize, y_crop: usize) {
        self.bus.tia.set_video_calibration(visible_start_clock, x_adjust, y_crop);
        log::info!("video: visible_start={} x_adjust={} y_crop={}", visible_start_clock, x_adjust, y_crop);
    }
    pub fn reset(&mut self) { self.cpu = Cpu6502::new(); self.cpu.reset(&mut self.bus); log::info!("RESET => PC=${:04X}", self.cpu.pc); }
    pub fn run_frame(&mut self) {
        self.bus.input = self.input;
        self.bus.trace = self.trace;
        let mut done = false;
        let mut guard = 0;
        while !done && guard < 80_000 {
            let cycles = self.cpu.step(&mut self.bus);
            if self.trace { log::trace!("{}", self.cpu.trace_line()); }
            done = self.bus.tick(cycles);
            // Atualiza o mixer durante o frame, não só no final. Muitas ROMs
            // ligam/desligam AUDV rapidamente; capturar só o estado final pode
            // deixar o som mudo.
            if let Ok(mut audio) = self.audio_state.lock() {
                let keep_test = audio.test_beep;
                *audio = self.bus.tia.audio_state();
                audio.test_beep = keep_test;
            }
            guard += cycles;
            if self.cpu.stopped { break; }
        }
        if guard >= 80_000 { log::warn!("frame guard atingido; possível loop/timing inconsistente"); }
        if let Ok(mut audio) = self.audio_state.lock() {
            let keep_test = audio.test_beep;
            *audio = self.bus.tia.audio_state();
            audio.test_beep = keep_test;
        }
    }
    pub fn framebuffer(&self) -> &[u8] { &self.bus.tia.frame }
}
