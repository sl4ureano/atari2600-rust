bitflags::bitflags! { #[derive(Clone, Copy)] pub struct Flags: u8 { const C=1; const Z=2; const I=4; const D=8; const B=16; const U=32; const V=64; const N=128; } }

pub trait Memory { fn read(&mut self, addr:u16)->u8; fn write(&mut self, addr:u16, val:u8); }

#[derive(Clone)]
pub struct Cpu6502 { pub a:u8,pub x:u8,pub y:u8,pub sp:u8,pub pc:u16,pub p:Flags,pub cycles:u64,pub stopped:bool,pub last_pc:u16,pub last_op:u8 }

impl Cpu6502 {
    pub fn new()->Self{Self{a:0,x:0,y:0,sp:0xfd,pc:0,p:Flags::I|Flags::U,cycles:0,stopped:false,last_pc:0,last_op:0}}
    pub fn reset<M:Memory>(&mut self, m:&mut M){ self.sp=0xfd; self.p=Flags::I|Flags::U; let lo=m.read(0xfffc) as u16; let hi=m.read(0xfffd) as u16; self.pc=(hi<<8)|lo; if self.pc==0 || self.pc==0xffff{self.pc=0xf000;} }
    pub fn trace_line(&self)->String{format!("PC={:04X} OP={:02X} A={:02X} X={:02X} Y={:02X} SP={:02X} P={:02X} CYC={}",self.last_pc,self.last_op,self.a,self.x,self.y,self.sp,self.p.bits(),self.cycles)}
    fn set_zn(&mut self,v:u8){ self.p.set(Flags::Z,v==0); self.p.set(Flags::N,v&0x80!=0); }
    fn fetch<M:Memory>(&mut self,m:&mut M)->u8{let v=m.read(self.pc);self.pc=self.pc.wrapping_add(1);v}
    fn zp<M:Memory>(&mut self,m:&mut M)->u16{self.fetch(m) as u16}
    fn zpx<M:Memory>(&mut self,m:&mut M)->u16{self.fetch(m).wrapping_add(self.x) as u16}
    fn zpy<M:Memory>(&mut self,m:&mut M)->u16{self.fetch(m).wrapping_add(self.y) as u16}
    fn abs<M:Memory>(&mut self,m:&mut M)->u16{let lo=self.fetch(m) as u16; let hi=self.fetch(m) as u16; (hi<<8)|lo}
    fn absx<M:Memory>(&mut self,m:&mut M)->u16{let a=self.abs(m);a.wrapping_add(self.x as u16)}
    fn absy<M:Memory>(&mut self,m:&mut M)->u16{let a=self.abs(m);a.wrapping_add(self.y as u16)}
    fn indx<M:Memory>(&mut self,m:&mut M)->u16{let p=self.fetch(m).wrapping_add(self.x); let lo=m.read(p as u16) as u16; let hi=m.read(p.wrapping_add(1) as u16) as u16; (hi<<8)|lo}
    fn indy<M:Memory>(&mut self,m:&mut M)->u16{let p=self.fetch(m); let lo=m.read(p as u16) as u16; let hi=m.read(p.wrapping_add(1) as u16) as u16; ((hi<<8)|lo).wrapping_add(self.y as u16)}
    fn push<M:Memory>(&mut self,m:&mut M,v:u8){m.write(0x0100|self.sp as u16,v);self.sp=self.sp.wrapping_sub(1)}
    fn pop<M:Memory>(&mut self,m:&mut M)->u8{self.sp=self.sp.wrapping_add(1);m.read(0x0100|self.sp as u16)}
    fn adc(&mut self,v:u8){let c=if self.p.contains(Flags::C){1}else{0};let sum=self.a as u16+v as u16+c;let r=sum as u8;self.p.set(Flags::C,sum>0xff);self.p.set(Flags::V,(!(self.a^v)&(self.a^r)&0x80)!=0);self.a=r;self.set_zn(self.a)}
    fn sbc(&mut self,v:u8){self.adc(!v)}
    fn cmp(&mut self,r:u8,v:u8){let x=r.wrapping_sub(v);self.p.set(Flags::C,r>=v);self.set_zn(x)}
    fn branch<M:Memory>(&mut self,m:&mut M,cond:bool)->u32{let off=self.fetch(m) as i8; if cond { self.pc=((self.pc as i32)+(off as i32)) as u16; 3 } else { 2 }}
    fn asl_val(&mut self,v:u8)->u8{self.p.set(Flags::C,v&0x80!=0);let r=v<<1;self.set_zn(r);r}
    fn lsr_val(&mut self,v:u8)->u8{self.p.set(Flags::C,v&1!=0);let r=v>>1;self.set_zn(r);r}
    fn rol_val(&mut self,v:u8)->u8{let c=if self.p.contains(Flags::C){1}else{0};self.p.set(Flags::C,v&0x80!=0);let r=(v<<1)|c;self.set_zn(r);r}
    fn ror_val(&mut self,v:u8)->u8{let c=if self.p.contains(Flags::C){0x80}else{0};self.p.set(Flags::C,v&1!=0);let r=(v>>1)|c;self.set_zn(r);r}

    pub fn step<M:Memory>(&mut self,m:&mut M)->u32{
        if self.stopped { return 1; }
        self.last_pc=self.pc; let op=self.fetch(m); self.last_op=op; let cyc:u32;
        match op {
            0x00=>{self.pc=self.pc.wrapping_add(1); self.push(m,(self.pc>>8)as u8);self.push(m,self.pc as u8);self.push(m,(self.p|Flags::B|Flags::U).bits());self.p.insert(Flags::I);let lo=m.read(0xfffe)as u16;let hi=m.read(0xffff)as u16;self.pc=(hi<<8)|lo;cyc=7}
            0xea=>cyc=2,
            // LDA/LDX/LDY
            0xa9=>{let v=self.fetch(m);self.a=v;self.set_zn(v);cyc=2} 0xa5=>{let a=self.zp(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=3} 0xb5=>{let a=self.zpx(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=4} 0xad=>{let a=self.abs(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=4} 0xbd=>{let a=self.absx(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=4} 0xb9=>{let a=self.absy(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=4} 0xa1=>{let a=self.indx(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=6} 0xb1=>{let a=self.indy(m);let v=m.read(a);self.a=v;self.set_zn(v);cyc=5}
            0xa2=>{let v=self.fetch(m);self.x=v;self.set_zn(v);cyc=2} 0xa6=>{let a=self.zp(m);let v=m.read(a);self.x=v;self.set_zn(v);cyc=3} 0xb6=>{let a=self.zpy(m);let v=m.read(a);self.x=v;self.set_zn(v);cyc=4} 0xae=>{let a=self.abs(m);let v=m.read(a);self.x=v;self.set_zn(v);cyc=4} 0xbe=>{let a=self.absy(m);let v=m.read(a);self.x=v;self.set_zn(v);cyc=4}
            0xa0=>{let v=self.fetch(m);self.y=v;self.set_zn(v);cyc=2} 0xa4=>{let a=self.zp(m);let v=m.read(a);self.y=v;self.set_zn(v);cyc=3} 0xb4=>{let a=self.zpx(m);let v=m.read(a);self.y=v;self.set_zn(v);cyc=4} 0xac=>{let a=self.abs(m);let v=m.read(a);self.y=v;self.set_zn(v);cyc=4} 0xbc=>{let a=self.absx(m);let v=m.read(a);self.y=v;self.set_zn(v);cyc=4}
            // STA/STX/STY
            0x85=>{let a=self.zp(m);m.write(a,self.a);cyc=3} 0x95=>{let a=self.zpx(m);m.write(a,self.a);cyc=4} 0x8d=>{let a=self.abs(m);m.write(a,self.a);cyc=4} 0x9d=>{let a=self.absx(m);m.write(a,self.a);cyc=5} 0x99=>{let a=self.absy(m);m.write(a,self.a);cyc=5} 0x81=>{let a=self.indx(m);m.write(a,self.a);cyc=6} 0x91=>{let a=self.indy(m);m.write(a,self.a);cyc=6}
            0x86=>{let a=self.zp(m);m.write(a,self.x);cyc=3} 0x96=>{let a=self.zpy(m);m.write(a,self.x);cyc=4} 0x8e=>{let a=self.abs(m);m.write(a,self.x);cyc=4}
            0x84=>{let a=self.zp(m);m.write(a,self.y);cyc=3} 0x94=>{let a=self.zpx(m);m.write(a,self.y);cyc=4} 0x8c=>{let a=self.abs(m);m.write(a,self.y);cyc=4}
            // Transfers/inc/dec
            0xaa=>{self.x=self.a;self.set_zn(self.x);cyc=2} 0xa8=>{self.y=self.a;self.set_zn(self.y);cyc=2} 0x8a=>{self.a=self.x;self.set_zn(self.a);cyc=2} 0x98=>{self.a=self.y;self.set_zn(self.a);cyc=2} 0xba=>{self.x=self.sp;self.set_zn(self.x);cyc=2} 0x9a=>{self.sp=self.x;cyc=2}
            0xe8=>{self.x=self.x.wrapping_add(1);self.set_zn(self.x);cyc=2} 0xc8=>{self.y=self.y.wrapping_add(1);self.set_zn(self.y);cyc=2} 0xca=>{self.x=self.x.wrapping_sub(1);self.set_zn(self.x);cyc=2} 0x88=>{self.y=self.y.wrapping_sub(1);self.set_zn(self.y);cyc=2}
            // ADC/SBC
            0x69=>{let v=self.fetch(m);self.adc(v);cyc=2} 0x65=>{let a=self.zp(m);let v=m.read(a);self.adc(v);cyc=3} 0x75=>{let a=self.zpx(m);let v=m.read(a);self.adc(v);cyc=4} 0x6d=>{let a=self.abs(m);let v=m.read(a);self.adc(v);cyc=4} 0x7d=>{let a=self.absx(m);let v=m.read(a);self.adc(v);cyc=4} 0x79=>{let a=self.absy(m);let v=m.read(a);self.adc(v);cyc=4} 0x61=>{let a=self.indx(m);let v=m.read(a);self.adc(v);cyc=6} 0x71=>{let a=self.indy(m);let v=m.read(a);self.adc(v);cyc=5}
            0xe9|0xeb=>{let v=self.fetch(m);self.sbc(v);cyc=2} 0xe5=>{let a=self.zp(m);let v=m.read(a);self.sbc(v);cyc=3} 0xf5=>{let a=self.zpx(m);let v=m.read(a);self.sbc(v);cyc=4} 0xed=>{let a=self.abs(m);let v=m.read(a);self.sbc(v);cyc=4} 0xfd=>{let a=self.absx(m);let v=m.read(a);self.sbc(v);cyc=4} 0xf9=>{let a=self.absy(m);let v=m.read(a);self.sbc(v);cyc=4} 0xe1=>{let a=self.indx(m);let v=m.read(a);self.sbc(v);cyc=6} 0xf1=>{let a=self.indy(m);let v=m.read(a);self.sbc(v);cyc=5}
            // CMP/CPX/CPY
            0xc9=>{let v=self.fetch(m);self.cmp(self.a,v);cyc=2} 0xc5=>{let a=self.zp(m);let v=m.read(a);self.cmp(self.a,v);cyc=3} 0xd5=>{let a=self.zpx(m);let v=m.read(a);self.cmp(self.a,v);cyc=4} 0xcd=>{let a=self.abs(m);let v=m.read(a);self.cmp(self.a,v);cyc=4} 0xdd=>{let a=self.absx(m);let v=m.read(a);self.cmp(self.a,v);cyc=4} 0xd9=>{let a=self.absy(m);let v=m.read(a);self.cmp(self.a,v);cyc=4} 0xc1=>{let a=self.indx(m);let v=m.read(a);self.cmp(self.a,v);cyc=6} 0xd1=>{let a=self.indy(m);let v=m.read(a);self.cmp(self.a,v);cyc=5}
            0xe0=>{let v=self.fetch(m);self.cmp(self.x,v);cyc=2} 0xe4=>{let a=self.zp(m);let v=m.read(a);self.cmp(self.x,v);cyc=3} 0xec=>{let a=self.abs(m);let v=m.read(a);self.cmp(self.x,v);cyc=4}
            0xc0=>{let v=self.fetch(m);self.cmp(self.y,v);cyc=2} 0xc4=>{let a=self.zp(m);let v=m.read(a);self.cmp(self.y,v);cyc=3} 0xcc=>{let a=self.abs(m);let v=m.read(a);self.cmp(self.y,v);cyc=4}
            // Logical ORA/AND/EOR
            0x09=>{let v=self.fetch(m);self.a|=v;self.set_zn(self.a);cyc=2} 0x05=>{let a=self.zp(m);self.a|=m.read(a);self.set_zn(self.a);cyc=3} 0x15=>{let a=self.zpx(m);self.a|=m.read(a);self.set_zn(self.a);cyc=4} 0x0d=>{let a=self.abs(m);self.a|=m.read(a);self.set_zn(self.a);cyc=4} 0x1d=>{let a=self.absx(m);self.a|=m.read(a);self.set_zn(self.a);cyc=4} 0x19=>{let a=self.absy(m);self.a|=m.read(a);self.set_zn(self.a);cyc=4} 0x01=>{let a=self.indx(m);self.a|=m.read(a);self.set_zn(self.a);cyc=6} 0x11=>{let a=self.indy(m);self.a|=m.read(a);self.set_zn(self.a);cyc=5}
            0x29=>{let v=self.fetch(m);self.a&=v;self.set_zn(self.a);cyc=2} 0x25=>{let a=self.zp(m);self.a&=m.read(a);self.set_zn(self.a);cyc=3} 0x35=>{let a=self.zpx(m);self.a&=m.read(a);self.set_zn(self.a);cyc=4} 0x2d=>{let a=self.abs(m);self.a&=m.read(a);self.set_zn(self.a);cyc=4} 0x3d=>{let a=self.absx(m);self.a&=m.read(a);self.set_zn(self.a);cyc=4} 0x39=>{let a=self.absy(m);self.a&=m.read(a);self.set_zn(self.a);cyc=4} 0x21=>{let a=self.indx(m);self.a&=m.read(a);self.set_zn(self.a);cyc=6} 0x31=>{let a=self.indy(m);self.a&=m.read(a);self.set_zn(self.a);cyc=5}
            0x49=>{let v=self.fetch(m);self.a^=v;self.set_zn(self.a);cyc=2} 0x45=>{let a=self.zp(m);self.a^=m.read(a);self.set_zn(self.a);cyc=3} 0x55=>{let a=self.zpx(m);self.a^=m.read(a);self.set_zn(self.a);cyc=4} 0x4d=>{let a=self.abs(m);self.a^=m.read(a);self.set_zn(self.a);cyc=4} 0x5d=>{let a=self.absx(m);self.a^=m.read(a);self.set_zn(self.a);cyc=4} 0x59=>{let a=self.absy(m);self.a^=m.read(a);self.set_zn(self.a);cyc=4} 0x41=>{let a=self.indx(m);self.a^=m.read(a);self.set_zn(self.a);cyc=6} 0x51=>{let a=self.indy(m);self.a^=m.read(a);self.set_zn(self.a);cyc=5}
            // BIT/JMP/JSR/RTS/RTI/Stack/Flags
            0x24=>{let a=self.zp(m);let v=m.read(a);self.p.set(Flags::Z,self.a&v==0);self.p.set(Flags::N,v&0x80!=0);self.p.set(Flags::V,v&0x40!=0);cyc=3} 0x2c=>{let a=self.abs(m);let v=m.read(a);self.p.set(Flags::Z,self.a&v==0);self.p.set(Flags::N,v&0x80!=0);self.p.set(Flags::V,v&0x40!=0);cyc=4}
            0x4c=>{self.pc=self.abs(m);cyc=3} 0x6c=>{let p=self.abs(m);let lo=m.read(p) as u16;let hi=m.read((p&0xff00)|((p+1)&0xff)) as u16;self.pc=(hi<<8)|lo;cyc=5} 0x20=>{let a=self.abs(m);let ret=self.pc.wrapping_sub(1);self.push(m,(ret>>8)as u8);self.push(m,ret as u8);self.pc=a;cyc=6} 0x60=>{let lo=self.pop(m) as u16;let hi=self.pop(m) as u16;self.pc=((hi<<8)|lo).wrapping_add(1);cyc=6}
            0x40=>{self.p=Flags::from_bits_truncate(self.pop(m));self.p.insert(Flags::U);let lo=self.pop(m) as u16;let hi=self.pop(m) as u16;self.pc=(hi<<8)|lo;cyc=6}
            0x48=>{self.push(m,self.a);cyc=3} 0x68=>{self.a=self.pop(m);self.set_zn(self.a);cyc=4} 0x08=>{self.push(m,(self.p|Flags::B|Flags::U).bits());cyc=3} 0x28=>{self.p=Flags::from_bits_truncate(self.pop(m));self.p.insert(Flags::U);cyc=4}
            0x18=>{self.p.remove(Flags::C);cyc=2} 0x38=>{self.p.insert(Flags::C);cyc=2} 0x58=>{self.p.remove(Flags::I);cyc=2} 0x78=>{self.p.insert(Flags::I);cyc=2} 0xb8=>{self.p.remove(Flags::V);cyc=2} 0xd8=>{self.p.remove(Flags::D);cyc=2} 0xf8=>{self.p.insert(Flags::D);cyc=2}
            // Branches
            0x10=>cyc=self.branch(m,!self.p.contains(Flags::N)), 0x30=>cyc=self.branch(m,self.p.contains(Flags::N)), 0x50=>cyc=self.branch(m,!self.p.contains(Flags::V)), 0x70=>cyc=self.branch(m,self.p.contains(Flags::V)), 0x90=>cyc=self.branch(m,!self.p.contains(Flags::C)), 0xb0=>cyc=self.branch(m,self.p.contains(Flags::C)), 0xd0=>cyc=self.branch(m,!self.p.contains(Flags::Z)), 0xf0=>cyc=self.branch(m,self.p.contains(Flags::Z)),
            // INC/DEC memory
            0xe6=>{let a=self.zp(m);let v=m.read(a).wrapping_add(1);m.write(a,v);self.set_zn(v);cyc=5} 0xf6=>{let a=self.zpx(m);let v=m.read(a).wrapping_add(1);m.write(a,v);self.set_zn(v);cyc=6} 0xee=>{let a=self.abs(m);let v=m.read(a).wrapping_add(1);m.write(a,v);self.set_zn(v);cyc=6} 0xfe=>{let a=self.absx(m);let v=m.read(a).wrapping_add(1);m.write(a,v);self.set_zn(v);cyc=7}
            0xc6=>{let a=self.zp(m);let v=m.read(a).wrapping_sub(1);m.write(a,v);self.set_zn(v);cyc=5} 0xd6=>{let a=self.zpx(m);let v=m.read(a).wrapping_sub(1);m.write(a,v);self.set_zn(v);cyc=6} 0xce=>{let a=self.abs(m);let v=m.read(a).wrapping_sub(1);m.write(a,v);self.set_zn(v);cyc=6} 0xde=>{let a=self.absx(m);let v=m.read(a).wrapping_sub(1);m.write(a,v);self.set_zn(v);cyc=7}
            // Shifts/rotates
            0x0a=>{self.a=self.asl_val(self.a);cyc=2} 0x06=>{let a=self.zp(m);let v=self.asl_val(m.read(a));m.write(a,v);cyc=5} 0x16=>{let a=self.zpx(m);let v=self.asl_val(m.read(a));m.write(a,v);cyc=6} 0x0e=>{let a=self.abs(m);let v=self.asl_val(m.read(a));m.write(a,v);cyc=6} 0x1e=>{let a=self.absx(m);let v=self.asl_val(m.read(a));m.write(a,v);cyc=7}
            0x4a=>{self.a=self.lsr_val(self.a);cyc=2} 0x46=>{let a=self.zp(m);let v=self.lsr_val(m.read(a));m.write(a,v);cyc=5} 0x56=>{let a=self.zpx(m);let v=self.lsr_val(m.read(a));m.write(a,v);cyc=6} 0x4e=>{let a=self.abs(m);let v=self.lsr_val(m.read(a));m.write(a,v);cyc=6} 0x5e=>{let a=self.absx(m);let v=self.lsr_val(m.read(a));m.write(a,v);cyc=7}
            0x2a=>{self.a=self.rol_val(self.a);cyc=2} 0x26=>{let a=self.zp(m);let v=self.rol_val(m.read(a));m.write(a,v);cyc=5} 0x36=>{let a=self.zpx(m);let v=self.rol_val(m.read(a));m.write(a,v);cyc=6} 0x2e=>{let a=self.abs(m);let v=self.rol_val(m.read(a));m.write(a,v);cyc=6} 0x3e=>{let a=self.absx(m);let v=self.rol_val(m.read(a));m.write(a,v);cyc=7}
            0x6a=>{self.a=self.ror_val(self.a);cyc=2} 0x66=>{let a=self.zp(m);let v=self.ror_val(m.read(a));m.write(a,v);cyc=5} 0x76=>{let a=self.zpx(m);let v=self.ror_val(m.read(a));m.write(a,v);cyc=6} 0x6e=>{let a=self.abs(m);let v=self.ror_val(m.read(a));m.write(a,v);cyc=6} 0x7e=>{let a=self.absx(m);let v=self.ror_val(m.read(a));m.write(a,v);cyc=7}
            // NOPs oficiais/ilegais comuns. Consome operandos para manter PC correto.
            0x1a|0x3a|0x5a|0x7a|0xda|0xfa=>cyc=2,
            0x80|0x82|0x89|0xc2|0xe2|0x04|0x44|0x64=>{let _=self.fetch(m);cyc=2}
            0x14|0x34|0x54|0x74|0xd4|0xf4=>{let _=self.fetch(m);cyc=4}
            0x0c=>{let _=self.abs(m);cyc=4}
            0x1c|0x3c|0x5c|0x7c|0xdc|0xfc=>{let _=self.absx(m);cyc=4}
            _=>{ log::error!("Opcode não implementado ${:02X} em ${:04X}",op,self.last_pc); self.stopped=true; cyc=1; }
        }
        self.cycles+=cyc as u64; cyc
    }
}
