use crate::{cartridge::Cartridge, cpu::Memory, input::Input, riot::Riot, tia::Tia};

pub struct Bus {
    pub tia: Tia,
    pub riot: Riot,
    pub cart: Cartridge,
    pub input: Input,
    pub trace: bool,
}

impl Bus {
    fn tia_read(&self, addr: u16) -> u8 {
        match addr & 0x0f {
            // INPT4: player 0 fire button, bit 7 active high when released.
            0x0c => self.input.inpt4,
            _ => self.tia.read(addr),
        }
    }

    pub fn new(cart: Cartridge) -> Self { Self { tia: Tia::new(), riot: Riot::new(), cart, input: Input::new(), trace: false } }
    pub fn tick(&mut self, cpu_cycles: u32) -> bool {
        self.riot.tick(cpu_cycles);
        self.tia.tick(cpu_cycles)
    }
}

impl Memory for Bus {
    fn read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x1fff; // 6507 usa 13 linhas de endereço
        match a {
            0x0000..=0x007f => self.tia_read(a),
            0x0080..=0x00ff => self.riot.read_ram(a),
            0x0100..=0x017f => self.tia_read(a),
            0x0180..=0x01ff => self.riot.read_ram(a),
            0x0280..=0x029f => self.riot.read_io(a, &self.input),
            0x1000..=0x1fff => self.cart.read(a),
            _ => match a & 0x1080 {
                0x0000 => self.tia_read(a),
                0x0080 => self.riot.read_ram(a),
                0x1000 => self.cart.read(a),
                _ => 0xff,
            }
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        let a = addr & 0x1fff;
        match a {
            0x0000..=0x007f => { if self.trace { log::trace!("TIA WRITE ${:02X} <= ${:02X}", a & 0x3f, val); } self.tia.write(a, val) },
            0x0080..=0x00ff => self.riot.write_ram(a, val),
            0x0100..=0x017f => { if self.trace { log::trace!("TIA WRITE ${:02X} <= ${:02X}", a & 0x3f, val); } self.tia.write(a, val) },
            0x0180..=0x01ff => self.riot.write_ram(a, val),
            0x0280..=0x029f => self.riot.write_io(a, val),
            0x1000..=0x1fff => self.cart.write(a, val),
            _ => match a & 0x1080 {
                0x0000 => self.tia.write(a, val),
                0x0080 => self.riot.write_ram(a, val),
                0x1000 => self.cart.write(a, val),
                _ => {}
            }
        }
    }
}
