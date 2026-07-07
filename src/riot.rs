use crate::input::Input;

pub struct Riot { ram: [u8; 128], timer: u8, interval: u32, ticks: u32 }
impl Riot {
    pub fn new() -> Self { Self { ram: [0;128], timer: 0, interval: 1, ticks: 0 } }
    pub fn read_ram(&self, addr: u16) -> u8 { self.ram[(addr as usize) & 0x7f] }
    pub fn write_ram(&mut self, addr: u16, val: u8) { self.ram[(addr as usize) & 0x7f] = val; }
    pub fn read_io(&self, addr: u16, input: &Input) -> u8 {
        match addr & 0x07 {
            0x00 => input.swcha,
            0x02 => input.swchb,
            0x04 => self.timer,
            _ => 0xff,
        }
    }
    pub fn write_io(&mut self, addr: u16, val: u8) {
        match addr & 0x17 {
            0x14 => { self.timer = val; self.interval = 1; }
            0x15 => { self.timer = val; self.interval = 8; }
            0x16 => { self.timer = val; self.interval = 64; }
            0x17 => { self.timer = val; self.interval = 1024; }
            _ => {}
        }
    }
    pub fn tick(&mut self, cycles: u32) {
        self.ticks += cycles;
        while self.ticks >= self.interval { self.ticks -= self.interval; self.timer = self.timer.wrapping_sub(1); }
    }
}
