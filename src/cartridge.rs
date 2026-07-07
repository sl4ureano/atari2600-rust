use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapper {
    Rom2K,
    Rom4K,
    F8,
    Unknown,
}

pub struct Cartridge {
    rom: Vec<u8>,
    bank: usize,
    mapper: Mapper,
}

impl Cartridge {
    pub fn new(mut rom: Vec<u8>) -> Result<Self> {
        if rom.is_empty() { return Err(anyhow!("ROM vazia")); }
        let mapper = match rom.len() {
            0..=2048 => { rom.resize(2048, 0xff); Mapper::Rom2K }
            2049..=4096 => { rom.resize(4096, 0xff); Mapper::Rom4K }
            4097..=8192 => { rom.resize(8192, 0xff); Mapper::F8 }
            _ => Mapper::Unknown,
        };

        // Em cartuchos F8, os vetores normalmente ficam visíveis no banco alto.
        let bank = if mapper == Mapper::F8 { 1 } else { 0 };
        Ok(Self { rom, bank, mapper })
    }

    pub fn len(&self) -> usize { self.rom.len() }
    pub fn mapper(&self) -> Mapper { self.mapper }
    pub fn bank(&self) -> usize { self.bank }

    pub fn read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x0fff;
        match self.mapper {
            Mapper::Rom2K => self.rom[(a as usize) & 0x07ff],
            Mapper::Rom4K => self.rom[(a as usize) & 0x0fff],
            Mapper::F8 => {
                // Hotspots F8: $1FF8 seleciona banco 0, $1FF9 seleciona banco 1.
                if a == 0x0ff8 { self.bank = 0; }
                if a == 0x0ff9 { self.bank = 1; }
                let off = self.bank * 4096 + ((a as usize) & 0x0fff);
                self.rom[off % self.rom.len()]
            }
            Mapper::Unknown => self.rom[(a as usize) % self.rom.len()],
        }
    }

    pub fn write(&mut self, addr: u16, _val: u8) {
        let a = addr & 0x0fff;
        if self.mapper == Mapper::F8 {
            if a == 0x0ff8 { self.bank = 0; }
            if a == 0x0ff9 { self.bank = 1; }
        }
    }
}
