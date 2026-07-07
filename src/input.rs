#[derive(Default, Clone, Copy)]
pub struct Input {
    /// SWCHA: joystick directions. Active low.
    pub swcha: u8,
    /// SWCHB: console switches. Active low for reset/select.
    pub swchb: u8,
    /// INPT4: player 0 fire button. Bit 7 high = released, low = pressed.
    pub inpt4: u8,
}

impl Input {
    pub fn new() -> Self { Self { swcha: 0xff, swchb: 0xff, inpt4: 0x80 } }

    pub fn set_left(&mut self, p: bool) { self.dir_bit(0x40, p); }
    pub fn set_right(&mut self, p: bool) { self.dir_bit(0x80, p); }
    pub fn set_up(&mut self, p: bool) { self.dir_bit(0x10, p); }
    pub fn set_down(&mut self, p: bool) { self.dir_bit(0x20, p); }
    pub fn set_fire(&mut self, p: bool) { self.inpt4 = if p { 0x00 } else { 0x80 }; }

    /// Select e Reset são botões momentâneos do console, não toggles.
    pub fn set_select(&mut self, p: bool) { self.console_bit(0x02, p); }
    pub fn set_reset_switch(&mut self, p: bool) { self.console_bit(0x01, p); }

    /// Color/BW. True = color, false = P/B.
    pub fn set_color(&mut self, color: bool) { if color { self.swchb |= 0x08 } else { self.swchb &= !0x08 } }
    pub fn toggle_color(&mut self) { self.swchb ^= 0x08; }

    /// Dificuldade dos jogadores. True = A, false = B.
    /// Nos switches do Atari, bits baixos/altos variam por revisão; para ROMs comuns,
    /// deixar high é o modo B/padrão e low força A.
    pub fn set_p0_difficulty_a(&mut self, a: bool) { if a { self.swchb &= !0x40 } else { self.swchb |= 0x40 } }
    pub fn set_p1_difficulty_a(&mut self, a: bool) { if a { self.swchb &= !0x80 } else { self.swchb |= 0x80 } }
    pub fn toggle_p0_difficulty(&mut self) { self.swchb ^= 0x40; }
    pub fn toggle_p1_difficulty(&mut self) { self.swchb ^= 0x80; }

    fn dir_bit(&mut self, mask: u8, pressed: bool) { if pressed { self.swcha &= !mask } else { self.swcha |= mask } }
    fn console_bit(&mut self, mask: u8, pressed: bool) { if pressed { self.swchb &= !mask } else { self.swchb |= mask } }
}
