pub const VISIBLE_WIDTH: usize = 160;
pub const VISIBLE_HEIGHT: usize = 192;
pub const FRAME_SIZE: usize = VISIBLE_WIDTH * VISIBLE_HEIGHT * 4;

// Timing NTSC aproximado. O Atari renderiza 228 color clocks por scanline;
// os primeiros ~68 ficam fora da área visível. A janela expõe 160x192.
const COLOR_CLOCKS_PER_LINE: usize = 228;
const DEFAULT_VISIBLE_START_CLOCK: usize = 68;

// Paleta NTSC aproximada com hue/luma. Registradores de cor do TIA usam
// high nibble para matiz e bits baixos pares para luminância. Esta tabela não
// substitui calibração de Stella, mas já aproxima melhor Enduro do que a
// paleta fake anterior.
const TIA_NTSC: [[[u8; 3]; 8]; 16] = [
    // 0x0: grayscale
    [[0,0,0],[36,36,36],[72,72,72],[108,108,108],[144,144,144],[176,176,176],[208,208,208],[236,236,236]],
    // 0x1: gold / dark yellow
    [[32,24,0],[64,48,0],[96,72,0],[128,96,0],[160,122,12],[194,154,42],[224,188,82],[248,220,132]],
    // 0x2: orange
    [[44,16,0],[80,34,0],[120,56,0],[160,82,10],[198,112,34],[226,144,70],[246,180,112],[255,214,160]],
    // 0x3: red-orange
    [[48,8,0],[86,22,0],[126,38,6],[166,58,22],[204,84,48],[232,120,86],[250,160,130],[255,202,178]],
    // 0x4: pink/red
    [[42,0,18],[82,0,40],[122,12,64],[162,36,92],[202,70,126],[230,110,164],[248,154,202],[255,198,232]],
    // 0x5: purple
    [[32,0,54],[66,12,92],[100,34,132],[134,64,172],[166,98,208],[198,136,232],[224,174,248],[246,214,255]],
    // 0x6: blue-violet
    [[12,10,72],[32,34,116],[56,66,160],[88,102,198],[122,138,228],[160,176,246],[198,210,255],[230,236,255]],
    // 0x7: blue
    [[0,24,86],[0,54,132],[10,90,178],[38,126,214],[76,160,238],[118,194,252],[164,222,255],[210,244,255]],
    // 0x8: sky blue / cyan-blue
    [[0,42,82],[0,76,126],[0,112,170],[24,148,208],[62,184,236],[106,214,252],[154,238,255],[204,252,255]],
    // 0x9: cyan/teal
    [[0,54,64],[0,88,98],[0,124,132],[18,160,166],[54,194,198],[96,222,226],[144,242,244],[196,254,254]],
    // 0xA: blue-green
    [[0,58,38],[0,92,58],[6,128,82],[34,164,110],[72,198,142],[116,224,176],[164,244,210],[212,255,238]],
    // 0xB: green
    [[0,62,0],[0,92,8],[10,124,20],[38,156,42],[72,188,70],[112,216,106],[160,238,152],[210,252,202]],
    // 0xC: yellow-green (Enduro road/grass uses this range in many dumps)
    [[20,54,0],[38,84,0],[62,116,0],[92,150,12],[126,184,36],[166,216,72],[206,240,116],[238,254,168]],
    // 0xD: olive/yellow
    [[44,44,0],[76,74,0],[110,106,0],[146,140,10],[184,176,34],[218,210,70],[244,236,112],[255,252,164]],
    // 0xE: warm yellow/orange
    [[54,34,0],[90,60,0],[128,88,0],[168,118,12],[206,152,40],[236,186,76],[252,220,124],[255,244,176]],
    // 0xF: tan/brown
    [[46,28,10],[80,52,24],[116,78,42],[154,106,64],[190,138,92],[222,174,126],[246,210,166],[255,238,210]],
];
#[derive(Clone)]
pub struct Tia {
    pub frame: Vec<u8>,
    back: Vec<u8>,

    /// Scanline física aproximada dentro do frame NTSC.
    pub scanline: usize,
    /// Linha visível já renderizada no framebuffer 160x192.
    visible_y: usize,

    pub color_bg: u8,
    pub color_pf: u8,
    pub color_p0: u8,
    pub color_p1: u8,

    pub audc0: u8,
    pub audf0: u8,
    pub audv0: u8,
    pub audc1: u8,
    pub audf1: u8,
    pub audv1: u8,

    nusiz0: u8,
    nusiz1: u8,
    pf0: u8,
    pf1: u8,
    pf2: u8,
    ctrlpf: u8,
    grp0: u8,
    grp1: u8,
    enabl: u8,
    enam0: u8,
    enam1: u8,

    hpos_p0: usize,
    hpos_p1: usize,
    hpos_m0: usize,
    hpos_m1: usize,
    hpos_bl: usize,

    hmp0: u8,
    hmp1: u8,
    hmm0: u8,
    hmm1: u8,
    hmbl: u8,

    vsync: bool,
    vblank: bool,
    cycles_in_line: usize,
    frame_ready: bool,

    // Registradores de colisão TIA. Bits são latched até CXCLR ($2C).
    cxm0p: u8,
    cxm1p: u8,
    cxp0fb: u8,
    cxp1fb: u8,
    cxm0fb: u8,
    cxm1fb: u8,
    cxblpf: u8,
    cxppmm: u8,

    had_visible_lines: bool,
    visible_start_clock: usize,
    x_adjust: isize,
    y_crop: usize,
}

impl Tia {
    pub fn new() -> Self {
        let mut tia = Self {
            frame: vec![0; FRAME_SIZE],
            back: vec![0; FRAME_SIZE],
            scanline: 0,
            visible_y: 0,
            color_bg: 0,
            color_pf: 0x0f,
            color_p0: 0x3f,
            color_p1: 0x8f,
            audc0: 0,
            audf0: 0,
            audv0: 0,
            audc1: 0,
            audf1: 0,
            audv1: 0,
            nusiz0: 0,
            nusiz1: 0,
            pf0: 0,
            pf1: 0,
            pf2: 0,
            ctrlpf: 0,
            grp0: 0,
            grp1: 0,
            enabl: 0,
            enam0: 0,
            enam1: 0,
            hpos_p0: 30,
            hpos_p1: 100,
            hpos_m0: 40,
            hpos_m1: 110,
            hpos_bl: 80,
            hmp0: 0,
            hmp1: 0,
            hmm0: 0,
            hmm1: 0,
            hmbl: 0,
            vsync: false,
            vblank: true,
            cycles_in_line: 0,
            frame_ready: false,
            cxm0p: 0,
            cxm1p: 0,
            cxp0fb: 0,
            cxp1fb: 0,
            cxm0fb: 0,
            cxm1fb: 0,
            cxblpf: 0,
            cxppmm: 0,
            had_visible_lines: false,
            visible_start_clock: DEFAULT_VISIBLE_START_CLOCK,
            x_adjust: -4,
            y_crop: 0,
        };
        tia.clear_back();
        tia.frame.copy_from_slice(&tia.back);
        tia
    }

    pub fn set_video_calibration(&mut self, visible_start_clock: usize, x_adjust: isize, y_crop: usize) {
        self.visible_start_clock = visible_start_clock.min(COLOR_CLOCKS_PER_LINE - VISIBLE_WIDTH);
        self.x_adjust = x_adjust;
        self.y_crop = y_crop.min(40);
    }


    pub fn take_frame_ready(&mut self) -> bool {
        let ready = self.frame_ready;
        self.frame_ready = false;
        ready
    }

    pub fn audio_state(&self) -> crate::audio::TiaAudioState {
        crate::audio::TiaAudioState {
            audc0: self.audc0, audf0: self.audf0, audv0: self.audv0,
            audc1: self.audc1, audf1: self.audf1, audv1: self.audv1,
            test_beep: false,
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr & 0x0f {
            // Collision latches. No TIA real os bits úteis ficam em 7/6.
            0x00 => self.cxm0p,
            0x01 => self.cxm1p,
            0x02 => self.cxp0fb,
            0x03 => self.cxp1fb,
            0x04 => self.cxm0fb,
            0x05 => self.cxm1fb,
            0x06 => self.cxblpf,
            0x07 => self.cxppmm,
            // INPT0..INPT3 paddles não implementados: linha em repouso.
            0x08..=0x0b => 0x80,
            // INPT4/5 são tratados no Bus para joystick fire.
            0x0c..=0x0d => 0x80,
            _ => 0x00,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr & 0x3f {
            // VSYNC. O frame só é publicado quando a ROM começa o VSYNC seguinte
            // depois de já termos renderizado linhas visíveis. Isso reduz muito rasgo/tela preta.
            0x00 => {
                let new_vsync = val & 0x02 != 0;
                if new_vsync && !self.vsync {
                    if self.had_visible_lines {
                        self.publish_frame();
                    }
                    // Início de um novo frame físico. Limpar aqui evita que linhas de frames
                    // anteriores fiquem acumuladas quando a ROM renderiza menos/mais de 192 linhas
                    // ou quando o kernel reposiciona a pista a cada frame, como em Enduro.
                    self.clear_back();
                    self.scanline = 0;
                    self.visible_y = 0;
                    self.cycles_in_line = 0;
                    self.had_visible_lines = false;
                }
                self.vsync = new_vsync;
            }
            // VBLANK
            0x01 => {
                let new_vblank = val & 0x02 != 0;
                if self.vblank && !new_vblank {
                    // Saída do VBLANK: começa a área visível do kernel.
                    self.visible_y = 0;
                }
                self.vblank = new_vblank;
            }
            // WSYNC: aproxima stall até a próxima scanline.
            0x02 => self.force_next_scanline(),
            // RSYNC: realinha horizontalmente.
            0x03 => self.cycles_in_line = 0,

            0x04 => self.nusiz0 = val,
            0x05 => self.nusiz1 = val,
            0x06 => self.color_p0 = val,
            0x07 => self.color_p1 = val,
            0x08 => self.color_pf = val,
            0x09 => self.color_bg = val,
            0x0a => self.ctrlpf = val,
            0x15 => { self.audc0 = val & 0x0f; log::debug!("AUDC0={:X}", self.audc0); },
            0x16 => { self.audc1 = val & 0x0f; log::debug!("AUDC1={:X}", self.audc1); },
            0x17 => { self.audf0 = val & 0x1f; log::debug!("AUDF0={}", self.audf0); },
            0x18 => { self.audf1 = val & 0x1f; log::debug!("AUDF1={}", self.audf1); },
            0x19 => { self.audv0 = val & 0x0f; log::debug!("AUDV0={}", self.audv0); },
            0x1a => { self.audv1 = val & 0x0f; log::debug!("AUDV1={}", self.audv1); },
            0x0d => self.pf0 = val,
            0x0e => self.pf1 = val,
            0x0f => self.pf2 = val,
            0x1b => self.grp0 = val,
            0x1c => self.grp1 = val,
            0x1d => self.enam0 = val,
            0x1e => self.enam1 = val,
            0x1f => self.enabl = val,

            // RESPx/RESMx/RESBL. A posição real depende do instante exato no color clock.
            0x10 => self.hpos_p0 = self.current_x(),
            0x11 => self.hpos_p1 = self.current_x(),
            0x12 => self.hpos_m0 = self.current_x(),
            0x13 => self.hpos_m1 = self.current_x(),
            0x14 => self.hpos_bl = self.current_x(),

            // Motion registers.
            0x20 => self.hmp0 = val,
            0x21 => self.hmp1 = val,
            0x22 => self.hmm0 = val,
            0x23 => self.hmm1 = val,
            0x24 => self.hmbl = val,

            // HMOVE aplica os registradores de movimento. No hardware há um bug/blank nos 8 pixels
            // iniciais; aqui priorizamos estabilidade visual.
            0x2a => self.apply_hmove(),
            // HMCLR
            0x2b => { self.hmp0 = 0; self.hmp1 = 0; self.hmm0 = 0; self.hmm1 = 0; self.hmbl = 0; },
            // CXCLR limpa todos os latches de colisão. Sem isso jogos como Enduro
            // não conseguem detectar batida corretamente entre frames.
            0x2c => self.clear_collisions(),
            _ => {}
        }
    }

    /// Avança a TIA em color clocks. Cada ciclo do 6507 corresponde a 3
    /// color clocks da TIA. Ao contrário da versão anterior, cada pixel é
    /// produzido no instante em que o feixe passa por ele; portanto escritas
    /// em PFx/GRPx/COLUx durante a scanline afetam somente os pixels seguintes.
    pub fn tick(&mut self, cpu_cycles: u32) -> bool {
        self.frame_ready = false;
        for _ in 0..cpu_cycles {
            self.tick_cpu_cycle();
        }
        self.frame_ready
    }

    pub fn tick_cpu_cycle(&mut self) {
        for _ in 0..3 {
            self.tick_color_clock();
        }
    }

    fn tick_color_clock(&mut self) {
        let clock = self.cycles_in_line;

        if !self.vsync && !self.vblank
            && clock >= self.visible_start_clock
            && clock < self.visible_start_clock + VISIBLE_WIDTH
        {
            let x = clock - self.visible_start_clock;
            if self.visible_y >= self.y_crop {
                let y = self.visible_y - self.y_crop;
                if y < VISIBLE_HEIGHT {
                    self.render_pixel(x, y);
                    self.had_visible_lines = true;
                }
            }
        }

        self.cycles_in_line += 1;
        if self.cycles_in_line >= COLOR_CLOCKS_PER_LINE {
            self.end_scanline();
        }
    }

    fn force_next_scanline(&mut self) {
        // WSYNC bloqueia o 6507 até o fim da scanline. Como a CPU deste
        // projeto ainda trabalha por instrução, consumimos aqui os color
        // clocks restantes para preservar o conteúdo já desenhado na linha.
        while self.cycles_in_line != 0 {
            self.tick_color_clock();
        }
    }

    fn end_scanline(&mut self) {
        self.cycles_in_line = 0;

        if !self.vsync && !self.vblank {
            self.visible_y += 1;
        }

        self.scanline += 1;
        if self.scanline >= 262 {
            // Fallback para ROMs que não geram VSYNC perfeito.
            if self.had_visible_lines {
                self.publish_frame();
            }
            self.clear_back();
            self.scanline = 0;
            self.visible_y = 0;
            self.had_visible_lines = false;
        }
    }

    fn publish_frame(&mut self) {
        self.frame.copy_from_slice(&self.back);
        self.frame_ready = true;
    }

    fn current_x(&self) -> usize {
        // Converte color clocks físicos para pixels visíveis. No TIA, RESPx durante o kernel
        // reposiciona o objeto no ponto horizontal corrente. Escritas antes da área visível
        // caem em 0; escritas depois são dobradas para dentro da largura visível para evitar
        // posições travadas na borda direita.
        let raw = self.cycles_in_line.saturating_sub(self.visible_start_clock) as isize + self.x_adjust;
        raw.rem_euclid(VISIBLE_WIDTH as isize) as usize
    }

    fn render_pixel(&mut self, x: usize, y: usize) {
        let pf = self.playfield_bit(x);
        let p0 = sprite_copies_bit(self.grp0, self.nusiz0, x, self.hpos_p0);
        let p1 = sprite_copies_bit(self.grp1, self.nusiz1, x, self.hpos_p1);
        let m0 = self.enam0 & 0x02 != 0 && missile_bit(self.nusiz0, x, self.hpos_m0);
        let m1 = self.enam1 & 0x02 != 0 && missile_bit(self.nusiz1, x, self.hpos_m1);
        let bl = self.enabl & 0x02 != 0
            && x >= self.hpos_bl
            && x < self.hpos_bl.saturating_add(ball_width(self.ctrlpf));

        self.latch_collisions(p0, p1, m0, m1, bl, pf);

        let score_mode = self.ctrlpf & 0x02 != 0;
        let priority_pf = self.ctrlpf & 0x04 != 0;

        let mut c = self.color_bg;
        if pf || bl {
            c = if score_mode && x < 80 {
                self.color_p0
            } else if score_mode {
                self.color_p1
            } else {
                self.color_pf
            };
        }

        if !priority_pf {
            if p0 || m0 { c = self.color_p0; }
            if p1 || m1 { c = self.color_p1; }
        } else if !(pf || bl) {
            if p0 || m0 { c = self.color_p0; }
            if p1 || m1 { c = self.color_p1; }
        }

        self.put_px(x, y, c);
    }

    fn playfield_bit(&self, x: usize) -> bool {
        let half = if x < 80 {
            x / 4
        } else if self.ctrlpf & 0x01 != 0 {
            (159 - x) / 4
        } else {
            (x - 80) / 4
        };

        match half {
            // PF0 aparece com bits 4..7.
            0..=3 => self.pf0 & (0x10 << half) != 0,
            // PF1 aparece invertido: bit 7 à esquerda.
            4..=11 => self.pf1 & (0x80 >> (half - 4)) != 0,
            // PF2 aparece bit 0 à esquerda.
            12..=19 => self.pf2 & (1 << (half - 12)) != 0,
            _ => false,
        }
    }

    fn latch_collisions(&mut self, p0: bool, p1: bool, m0: bool, m1: bool, bl: bool, pf: bool) {
        // Layout dos registradores TIA:
        // CXM0P:  bit7 M0-P1, bit6 M0-P0
        // CXM1P:  bit7 M1-P0, bit6 M1-P1
        // CXP0FB: bit7 P0-PF, bit6 P0-BL
        // CXP1FB: bit7 P1-PF, bit6 P1-BL
        // CXM0FB: bit7 M0-PF, bit6 M0-BL
        // CXM1FB: bit7 M1-PF, bit6 M1-BL
        // CXBLPF: bit7 BL-PF
        // CXPPMM: bit7 P0-P1, bit6 M0-M1
        if m0 && p1 { self.cxm0p |= 0x80; }
        if m0 && p0 { self.cxm0p |= 0x40; }
        if m1 && p0 { self.cxm1p |= 0x80; }
        if m1 && p1 { self.cxm1p |= 0x40; }
        if p0 && pf { self.cxp0fb |= 0x80; }
        if p0 && bl { self.cxp0fb |= 0x40; }
        if p1 && pf { self.cxp1fb |= 0x80; }
        if p1 && bl { self.cxp1fb |= 0x40; }
        if m0 && pf { self.cxm0fb |= 0x80; }
        if m0 && bl { self.cxm0fb |= 0x40; }
        if m1 && pf { self.cxm1fb |= 0x80; }
        if m1 && bl { self.cxm1fb |= 0x40; }
        if bl && pf { self.cxblpf |= 0x80; }
        if p0 && p1 { self.cxppmm |= 0x80; }
        if m0 && m1 { self.cxppmm |= 0x40; }
    }

    fn clear_collisions(&mut self) {
        self.cxm0p = 0;
        self.cxm1p = 0;
        self.cxp0fb = 0;
        self.cxp1fb = 0;
        self.cxm0fb = 0;
        self.cxm1fb = 0;
        self.cxblpf = 0;
        self.cxppmm = 0;
    }

    fn put_px(&mut self, x: usize, y: usize, color: u8) {
        let [r, g, b] = tia_color_rgb(color);
        let i = (y * VISIBLE_WIDTH + x) * 4;
        self.back[i] = r;
        self.back[i + 1] = g;
        self.back[i + 2] = b;
        self.back[i + 3] = 0xff;
    }

    fn clear_back(&mut self) {
        for px in self.back.chunks_exact_mut(4) {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
            px[3] = 0xff;
        }
    }

    fn apply_hmove(&mut self) {
        self.hpos_p0 = move_h(self.hpos_p0, self.hmp0);
        self.hpos_p1 = move_h(self.hpos_p1, self.hmp1);
        self.hpos_m0 = move_h(self.hpos_m0, self.hmm0);
        self.hpos_m1 = move_h(self.hpos_m1, self.hmm1);
        self.hpos_bl = move_h(self.hpos_bl, self.hmbl);
    }
}

fn sprite_copies_bit(g: u8, nusiz: u8, x: usize, base: usize) -> bool {
    let scale = player_scale(nusiz);
    for off in player_offsets(nusiz) {
        if sprite_bit_scaled(g, x.wrapping_sub(base.wrapping_add(*off)), scale) { return true; }
    }
    false
}

fn sprite_bit_scaled(g: u8, rel: usize, scale: usize) -> bool {
    let bit = rel / scale;
    bit < 8 && (g & (0x80 >> bit)) != 0
}

fn missile_bit(nusiz: u8, x: usize, base: usize) -> bool {
    let w = missile_width(nusiz);
    for off in missile_offsets(nusiz) {
        let start = base.wrapping_add(*off);
        if x >= start && x < start.saturating_add(w) { return true; }
    }
    false
}

fn player_offsets(nusiz: u8) -> &'static [usize] {
    // NUSIZ bits 0..2 control player copies/size. The previous version treated
    // values 5 and 7 as extra copies; on real TIA they are size modes. That
    // made Enduro draw duplicate road/player kernels.
    match nusiz & 0x07 {
        0 => &[0],           // one copy
        1 => &[0, 16],       // two close copies
        2 => &[0, 32],       // two medium copies
        3 => &[0, 16, 32],   // three close copies
        4 => &[0, 64],       // two wide copies
        5 => &[0],           // double-size player
        6 => &[0, 32, 64],   // three medium copies
        7 => &[0],           // quad-size player
        _ => &[0],
    }
}

fn missile_offsets(nusiz: u8) -> &'static [usize] {
    // Missiles follow the copy count encoded by NUSIZ, but size modes 5/7 do not
    // create fake extra copies. Width still comes from bits 4..5.
    match nusiz & 0x07 {
        0 => &[0],
        1 => &[0, 16],
        2 => &[0, 32],
        3 => &[0, 16, 32],
        4 => &[0, 64],
        5 => &[0],
        6 => &[0, 32, 64],
        7 => &[0],
        _ => &[0],
    }
}

fn player_scale(nusiz: u8) -> usize {
    match nusiz & 0x07 {
        5 => 2,
        7 => 4,
        _ => 1,
    }
}

fn missile_width(nusiz: u8) -> usize {
    match (nusiz >> 4) & 0x03 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => 8,
    }
}

fn ball_width(ctrlpf: u8) -> usize {
    match (ctrlpf >> 4) & 0x03 {
        0 => 1,
        1 => 2,
        2 => 4,
        _ => 8,
    }
}

fn move_h(pos: usize, val: u8) -> usize {
    // HMPx usa nibble alto em complemento de dois. Sinal invertido em relação ao que parece intuitivo:
    // valores positivos movem para a esquerda no TIA.
    let nibble = (val >> 4) as i8;
    let signed = if nibble & 0x08 != 0 { (nibble as i16) - 16 } else { nibble as i16 };
    ((pos as i16 - signed).rem_euclid(VISIBLE_WIDTH as i16)) as usize
}

fn tia_color_rgb(color: u8) -> [u8; 3] {
    let hue = ((color >> 4) & 0x0f) as usize;
    let lum = ((color >> 1) & 0x07) as usize;
    TIA_NTSC[hue][lum]
}
