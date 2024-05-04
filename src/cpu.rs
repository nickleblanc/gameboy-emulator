mod flags_register;
mod instructions;
mod registers;

use super::mmu::Memory;
// use crate::mmu::Memory;

use std::fs::OpenOptions;
use std::io::Write;

use instructions::{
    ADDHLTarget, ArithmeticTarget, BitPosition, IncDecTarget, IndirectTarget, Instruction,
    JumpCondition, LoadByteSource, LoadByteTarget, LoadType, LoadWordTarget, PrefixTarget,
    StackTarget,
};
pub struct CPU {
    registers: registers::Registers,
    sp: u16,
    pc: u16,
    pub mem: Memory,
    ime: bool,
    is_halted: bool,
}

impl CPU {
    pub fn new(mem: Memory) -> CPU {
        CPU {
            registers: registers::Registers::new(),
            sp: 0xFFFE,
            pc: 0x100,
            mem,
            ime: true,
            is_halted: false,
        }
    }

    pub fn load(&mut self, data: Vec<u8>) {
        for (i, byte) in data.iter().enumerate() {
            self.mem.write_byte(i as u16, *byte);
        }
    }

    fn log(&self) {
        let string = format!(
            "A: {:02x?} F: {:02X?} B: {:02X?} C: {:02X?} D: {:02X?} E: {:02X?} H: {:02X?} L: {:02X?} SP: {:04X?} PC: {:04X?} ({:02X?} {:02X?} {:02X?} {:02X?})\n",
            self.registers.a,
            u8::from(self.registers.f), self.registers.b, self.registers.c, self.registers.d, self.registers.e, self.registers.h, self.registers.l, self.sp, self.pc, self.mem.read_byte(self.pc.wrapping_add(1)), self.mem.read_byte(self.pc.wrapping_add(2)), self.mem.read_byte(self.pc.wrapping_add(3)), self.mem.read_byte(self.pc.wrapping_add(4))
        );

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open("log.txt")
            .expect("failed to open log file");

        file.write_all(string.as_bytes())
            .expect("failed to write to log");
    }

    pub fn step(&mut self) -> u8 {
        if self.mem.read_byte(0xFF02) == 0x81 {
            print!("{}", self.mem.read_byte(0xFF01) as char);
            self.mem.write_byte(0xFF02, 0);
        }

        // self.log();

        let mut instruction_byte = self.mem.read_byte(self.pc);

        let prefixed = instruction_byte == 0xCB;
        if prefixed {
            instruction_byte = self.read_next_byte();
        }

        let instruction = Instruction::get_instruction(instruction_byte, prefixed);
        let (next_pc, mut cycles) = self.execute(instruction);

        self.mem.step(cycles);

        if self.mem.interrupt_called() {
            self.is_halted = false;
        }
        if !self.is_halted {
            self.pc = next_pc;
        }

        let mut interrupted = false;
        if self.ime {
            if self.mem.interrupt_enable.vblank && self.mem.interrupt_flags.vblank {
                interrupted = true;
                self.mem.interrupt_flags.vblank = false;
                self.interrupt(0x40);
            }
            if self.mem.interrupt_enable.lcd_stat && self.mem.interrupt_flags.lcd_stat {
                interrupted = true;
                self.mem.interrupt_flags.lcd_stat = false;
                self.interrupt(0x48);
            }
            if self.mem.interrupt_enable.timer && self.mem.interrupt_flags.timer {
                interrupted = true;
                self.mem.interrupt_flags.timer = false;
                self.interrupt(0x50);
            }
        }
        if interrupted {
            cycles += 12
        }
        cycles
    }

    fn interrupt(&mut self, location: u16) {
        self.ime = false;
        self.push(self.pc);
        self.pc = location;
        self.mem.step(12);
    }

    fn execute(&mut self, instruction: Instruction) -> (u16, u8) {
        match instruction {
            Instruction::ADC(target) => {
                match target {
                    ArithmeticTarget::A => self.adc(self.registers.a),
                    ArithmeticTarget::B => self.adc(self.registers.b),
                    ArithmeticTarget::C => self.adc(self.registers.c),
                    ArithmeticTarget::D => self.adc(self.registers.d),
                    ArithmeticTarget::E => self.adc(self.registers.e),
                    ArithmeticTarget::H => self.adc(self.registers.h),
                    ArithmeticTarget::L => self.adc(self.registers.l),
                    ArithmeticTarget::HL => self.adc(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.adc(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::ADD(target) => {
                match target {
                    ArithmeticTarget::A => self.add(self.registers.a),
                    ArithmeticTarget::B => self.add(self.registers.b),
                    ArithmeticTarget::C => self.add(self.registers.c),
                    ArithmeticTarget::D => self.add(self.registers.d),
                    ArithmeticTarget::E => self.add(self.registers.e),
                    ArithmeticTarget::H => self.add(self.registers.h),
                    ArithmeticTarget::L => self.add(self.registers.l),
                    ArithmeticTarget::HL => self.add(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.add(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::ADDHL(target) => {
                match target {
                    ADDHLTarget::BC => self.add_hl(self.registers.get_bc()),
                    ADDHLTarget::DE => self.add_hl(self.registers.get_de()),
                    ADDHLTarget::HL => self.add_hl(self.registers.get_hl()),
                    ADDHLTarget::SP => self.add_hl(self.sp),
                }
                (self.pc.wrapping_add(1), 8)
            }
            Instruction::ADDSP => {
                let value = self.read_next_byte() as i8 as i16 as u16;
                let result = self.sp.wrapping_add(value);
                self.registers.f.z = false;
                self.registers.f.n = false;
                self.registers.f.h = (self.sp & 0xF) + (value & 0xF) > 0xF;
                self.registers.f.c = (self.sp & 0xFF) + (value & 0xFF) > 0xFF;
                self.sp = result;
                (self.pc.wrapping_add(2), 8)
            }
            Instruction::AND(target) => {
                match target {
                    ArithmeticTarget::A => self.and(self.registers.a),
                    ArithmeticTarget::B => self.and(self.registers.b),
                    ArithmeticTarget::C => self.and(self.registers.c),
                    ArithmeticTarget::D => self.and(self.registers.d),
                    ArithmeticTarget::E => self.and(self.registers.e),
                    ArithmeticTarget::H => self.and(self.registers.h),
                    ArithmeticTarget::L => self.and(self.registers.l),
                    ArithmeticTarget::HL => self.and(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.and(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 8),
                }
            }
            Instruction::BIT(register, bit_position) => {
                match register {
                    PrefixTarget::A => self.test_bit(self.registers.a, bit_position),
                    PrefixTarget::B => self.test_bit(self.registers.b, bit_position),
                    PrefixTarget::C => self.test_bit(self.registers.c, bit_position),
                    PrefixTarget::D => self.test_bit(self.registers.d, bit_position),
                    PrefixTarget::E => self.test_bit(self.registers.e, bit_position),
                    PrefixTarget::H => self.test_bit(self.registers.h, bit_position),
                    PrefixTarget::L => self.test_bit(self.registers.l, bit_position),
                    PrefixTarget::HL => {
                        self.test_bit(self.mem.read_byte(self.registers.get_hl()), bit_position)
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 12,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::CALL(condition) => {
                let jump_condition = match condition {
                    JumpCondition::NotZero => !self.registers.f.z,
                    JumpCondition::Zero => self.registers.f.z,
                    JumpCondition::NotCarry => !self.registers.f.c,
                    JumpCondition::Carry => self.registers.f.c,
                    JumpCondition::Always => true,
                };
                self.call(jump_condition)
            }
            Instruction::CCF => {
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = !self.registers.f.c;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::CP(target) => {
                match target {
                    ArithmeticTarget::A => self.cp(self.registers.a),
                    ArithmeticTarget::B => self.cp(self.registers.b),
                    ArithmeticTarget::C => self.cp(self.registers.c),
                    ArithmeticTarget::D => self.cp(self.registers.d),
                    ArithmeticTarget::E => self.cp(self.registers.e),
                    ArithmeticTarget::H => self.cp(self.registers.h),
                    ArithmeticTarget::L => self.cp(self.registers.l),
                    ArithmeticTarget::HL => self.cp(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.cp(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::CPL => {
                self.registers.a = !self.registers.a;
                self.registers.f.n = true;
                self.registers.f.h = true;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::DAA => {
                let flags = self.registers.f;
                let mut carry = false;

                if !flags.n {
                    if flags.c || self.registers.a > 0x99 {
                        self.registers.a = self.registers.a.wrapping_add(0x60);
                        carry = true;
                    }
                    if flags.h || (self.registers.a & 0x0F) > 0x09 {
                        self.registers.a = self.registers.a.wrapping_add(0x06);
                    }
                } else if flags.c {
                    carry = true;
                    let add = if flags.h { 0x9A } else { 0xA0 };
                    self.registers.a = self.registers.a.wrapping_add(add);
                } else if flags.h {
                    self.registers.a = self.registers.a.wrapping_add(0xFA);
                }

                self.registers.f.z = self.registers.a == 0;
                self.registers.f.h = false;
                self.registers.f.c = carry;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::DEC(target) => {
                match target {
                    IncDecTarget::A => self.registers.a = self.dec(self.registers.a),
                    IncDecTarget::B => self.registers.b = self.dec(self.registers.b),
                    IncDecTarget::C => self.registers.c = self.dec(self.registers.c),
                    IncDecTarget::D => self.registers.d = self.dec(self.registers.d),
                    IncDecTarget::E => self.registers.e = self.dec(self.registers.e),
                    IncDecTarget::H => self.registers.h = self.dec(self.registers.h),
                    IncDecTarget::L => self.registers.l = self.dec(self.registers.l),
                    IncDecTarget::HLI => {
                        let hl = self.registers.get_hl();
                        let value = self.dec(self.mem.read_byte(hl));
                        self.mem.write_byte(hl, value);
                    }
                    IncDecTarget::BC => {
                        let value = self.dec_16bit(self.registers.get_bc());
                        self.registers.set_bc(value);
                    }
                    IncDecTarget::DE => {
                        let value = self.dec_16bit(self.registers.get_de());
                        self.registers.set_de(value);
                    }
                    IncDecTarget::HL => {
                        let value = self.dec_16bit(self.registers.get_hl());
                        self.registers.set_hl(value);
                    }
                    IncDecTarget::SP => self.sp = self.dec_16bit(self.sp),
                }
                let cycles = match target {
                    IncDecTarget::HLI => 12,
                    IncDecTarget::BC => 8,
                    IncDecTarget::DE => 8,
                    IncDecTarget::HL => 8,
                    IncDecTarget::SP => 8,
                    _ => 4,
                };
                (self.pc.wrapping_add(1), cycles)
            }
            Instruction::DI => {
                self.ime = false;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::EI => {
                self.ime = true;
                // self.mem.write_byte(0xFFFF, )
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::HALT => {
                self.is_halted = true;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::INC(target) => {
                match target {
                    IncDecTarget::A => self.registers.a = self.inc(self.registers.a),
                    IncDecTarget::B => self.registers.b = self.inc(self.registers.b),
                    IncDecTarget::C => self.registers.c = self.inc(self.registers.c),
                    IncDecTarget::D => self.registers.d = self.inc(self.registers.d),
                    IncDecTarget::E => self.registers.e = self.inc(self.registers.e),
                    IncDecTarget::H => self.registers.h = self.inc(self.registers.h),
                    IncDecTarget::L => self.registers.l = self.inc(self.registers.l),
                    IncDecTarget::HLI => {
                        let hl = self.registers.get_hl();
                        let value = self.inc(self.mem.read_byte(hl));
                        self.mem.write_byte(hl, value);
                    }
                    IncDecTarget::BC => {
                        let value = self.inc_16bit(self.registers.get_bc());
                        self.registers.set_bc(value);
                    }
                    IncDecTarget::DE => {
                        let value = self.inc_16bit(self.registers.get_de());
                        self.registers.set_de(value);
                    }
                    IncDecTarget::HL => {
                        let value = self.inc_16bit(self.registers.get_hl());
                        self.registers.set_hl(value);
                    }
                    IncDecTarget::SP => self.sp = self.inc_16bit(self.sp),
                }
                let cycles = match target {
                    IncDecTarget::HLI => 12,
                    IncDecTarget::BC => 8,
                    IncDecTarget::DE => 8,
                    IncDecTarget::HL => 8,
                    IncDecTarget::SP => 8,
                    _ => 4,
                };
                (self.pc.wrapping_add(1), cycles)
            }
            Instruction::JP(condition) => {
                let jump_condition = match condition {
                    JumpCondition::NotZero => !self.registers.f.z,
                    JumpCondition::Zero => self.registers.f.z,
                    JumpCondition::NotCarry => !self.registers.f.c,
                    JumpCondition::Carry => self.registers.f.c,
                    JumpCondition::Always => true,
                };
                self.jump(jump_condition)
            }
            Instruction::JPHL => (self.registers.get_hl(), 4),
            Instruction::JR(condition) => {
                let jump_condition = match condition {
                    JumpCondition::NotZero => !self.registers.f.z,
                    JumpCondition::Zero => self.registers.f.z,
                    JumpCondition::NotCarry => !self.registers.f.c,
                    JumpCondition::Carry => self.registers.f.c,
                    JumpCondition::Always => true,
                };
                self.jump_relative(jump_condition)
            }
            Instruction::LD(target) => match target {
                LoadType::Byte(target, source) => {
                    let source_value = match source {
                        LoadByteSource::A => self.registers.a,
                        LoadByteSource::B => self.registers.b,
                        LoadByteSource::C => self.registers.c,
                        LoadByteSource::D => self.registers.d,
                        LoadByteSource::E => self.registers.e,
                        LoadByteSource::H => self.registers.h,
                        LoadByteSource::L => self.registers.l,
                        LoadByteSource::D8 => self.read_next_byte(),
                        LoadByteSource::HL => self.mem.read_byte(self.registers.get_hl()),
                    };
                    match target {
                        LoadByteTarget::A => self.registers.a = source_value,
                        LoadByteTarget::B => self.registers.b = source_value,
                        LoadByteTarget::C => self.registers.c = source_value,
                        LoadByteTarget::D => self.registers.d = source_value,
                        LoadByteTarget::E => self.registers.e = source_value,
                        LoadByteTarget::H => self.registers.h = source_value,
                        LoadByteTarget::L => self.registers.l = source_value,
                        LoadByteTarget::HL => {
                            self.mem.write_byte(self.registers.get_hl(), source_value)
                        }
                    }
                    match source {
                        LoadByteSource::D8 => (self.pc.wrapping_add(2), 8),
                        LoadByteSource::HL => (self.pc.wrapping_add(1), 8),
                        _ => (self.pc.wrapping_add(1), 4),
                    }
                }
                LoadType::Word(target) => {
                    let word = self.read_next_word();
                    match target {
                        LoadWordTarget::BC => self.registers.set_bc(word),
                        LoadWordTarget::DE => self.registers.set_de(word),
                        LoadWordTarget::HL => self.registers.set_hl(word),
                        LoadWordTarget::SP => self.sp = word,
                    };
                    (self.pc.wrapping_add(3), 12)
                }
                LoadType::IndirectFromA(target) => {
                    let a = self.registers.a;
                    match target {
                        IndirectTarget::BC => self.mem.write_byte(self.registers.get_bc(), a),
                        IndirectTarget::DE => self.mem.write_byte(self.registers.get_de(), a),
                        IndirectTarget::HLI => {
                            let hl = self.registers.get_hl();
                            self.registers.set_hl(hl.wrapping_add(1));
                            self.mem.write_byte(hl, a);
                        }
                        IndirectTarget::HLD => {
                            let hl = self.registers.get_hl();
                            self.registers.set_hl(hl.wrapping_sub(1));
                            self.mem.write_byte(hl, a);
                        }
                        IndirectTarget::Word => {
                            let word = self.read_next_word();
                            self.mem.write_byte(word, a);
                        }
                        IndirectTarget::LastByte => {
                            let c = self.registers.c as u16;
                            self.mem.write_byte(0xFF00 + c, a);
                        }
                    }
                    match target {
                        IndirectTarget::Word => (self.pc.wrapping_add(3), 16),
                        _ => (self.pc.wrapping_add(1), 8),
                    }
                }
                LoadType::AFromIndirect(target) => {
                    self.registers.a = match target {
                        IndirectTarget::BC => self.mem.read_byte(self.registers.get_bc()),
                        IndirectTarget::DE => self.mem.read_byte(self.registers.get_de()),
                        IndirectTarget::HLI => {
                            let hl = self.registers.get_hl();
                            self.registers.set_hl(hl.wrapping_add(1));
                            self.mem.read_byte(hl)
                        }
                        IndirectTarget::HLD => {
                            let hl = self.registers.get_hl();
                            self.registers.set_hl(hl.wrapping_sub(1));
                            self.mem.read_byte(hl)
                        }
                        IndirectTarget::Word => {
                            let word = self.read_next_word();
                            self.mem.read_byte(word)
                        }
                        IndirectTarget::LastByte => {
                            let c = self.registers.c as u16;
                            self.mem.read_byte(0xFF00 + c)
                        }
                    };
                    match target {
                        IndirectTarget::Word => (self.pc.wrapping_add(3), 16),
                        _ => (self.pc.wrapping_add(1), 8),
                    }
                }
                LoadType::ByteAddressFromA => {
                    let offset = self.read_next_byte() as u16;
                    self.mem.write_byte(0xFF00 + offset, self.registers.a);
                    (self.pc.wrapping_add(2), 12)
                }
                LoadType::AFromByteAddress => {
                    let offset = self.read_next_byte() as u16;
                    self.registers.a = self.mem.read_byte(0xFF00 + offset);
                    (self.pc.wrapping_add(2), 12)
                }
                LoadType::HLFromSP => {
                    let offset = self.read_next_byte() as i8 as i16 as u16;
                    let result = self.sp.wrapping_add(offset);
                    self.registers.set_hl(result);
                    self.registers.f.z = false;
                    self.registers.f.n = false;
                    self.registers.f.h = (self.sp & 0xF) + (offset & 0xF) > 0xF;
                    self.registers.f.c = (self.sp & 0xFF) + (offset & 0xFF) > 0xFF;
                    (self.pc.wrapping_add(2), 12)
                }
                LoadType::SPFromHL => {
                    self.sp = self.registers.get_hl();
                    (self.pc.wrapping_add(1), 8)
                }
                LoadType::IndirectFromSP => {
                    let address = self.read_next_word();
                    let sp = self.sp;
                    self.mem.write_byte(address, (sp & 0xFF) as u8);
                    self.mem
                        .write_byte(address.wrapping_add(1), ((sp & 0xFF00) >> 8) as u8);
                    (self.pc.wrapping_add(3), 20)
                }
            },
            Instruction::NOP => (self.pc.wrapping_add(1), 4),
            Instruction::OR(target) => {
                match target {
                    ArithmeticTarget::A => self.or(self.registers.a),
                    ArithmeticTarget::B => self.or(self.registers.b),
                    ArithmeticTarget::C => self.or(self.registers.c),
                    ArithmeticTarget::D => self.or(self.registers.d),
                    ArithmeticTarget::E => self.or(self.registers.e),
                    ArithmeticTarget::H => self.or(self.registers.h),
                    ArithmeticTarget::L => self.or(self.registers.l),
                    ArithmeticTarget::HL => self.or(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.or(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::POP(target) => {
                let value = self.pop();
                match target {
                    StackTarget::AF => self.registers.set_af(value),
                    StackTarget::BC => self.registers.set_bc(value),
                    StackTarget::DE => self.registers.set_de(value),
                    StackTarget::HL => self.registers.set_hl(value),
                }
                (self.pc.wrapping_add(1), 12)
            }
            Instruction::PUSH(target) => {
                let value = match target {
                    StackTarget::AF => self.registers.get_af(),
                    StackTarget::BC => self.registers.get_bc(),
                    StackTarget::DE => self.registers.get_de(),
                    StackTarget::HL => self.registers.get_hl(),
                };
                self.push(value);
                (self.pc.wrapping_add(1), 16)
            }
            Instruction::RES(register, bit_position) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a = self.reset_bit(self.registers.a, bit_position)
                    }
                    PrefixTarget::B => {
                        self.registers.b = self.reset_bit(self.registers.b, bit_position)
                    }
                    PrefixTarget::C => {
                        self.registers.c = self.reset_bit(self.registers.c, bit_position)
                    }
                    PrefixTarget::D => {
                        self.registers.d = self.reset_bit(self.registers.d, bit_position)
                    }
                    PrefixTarget::E => {
                        self.registers.e = self.reset_bit(self.registers.e, bit_position)
                    }
                    PrefixTarget::H => {
                        self.registers.h = self.reset_bit(self.registers.h, bit_position)
                    }

                    PrefixTarget::L => {
                        self.registers.l = self.reset_bit(self.registers.l, bit_position)
                    }

                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.reset_bit(value, bit_position);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RET(condition) => {
                let jump_condition = match condition {
                    JumpCondition::NotZero => !self.registers.f.z,
                    JumpCondition::Zero => self.registers.f.z,
                    JumpCondition::NotCarry => !self.registers.f.c,
                    JumpCondition::Carry => self.registers.f.c,
                    JumpCondition::Always => true,
                };
                let next_pc = self.ret(jump_condition);

                let cycles = if jump_condition && condition == JumpCondition::Always {
                    16
                } else if jump_condition {
                    20
                } else {
                    8
                };
                (next_pc, cycles)
            }
            Instruction::RETI => {
                self.ime = true;
                (self.pop(), 16)
            }
            Instruction::RL(register) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a = self.rotate_left_through_carry_set_zero(self.registers.a)
                    }
                    PrefixTarget::C => {
                        self.registers.c = self.rotate_left_through_carry_set_zero(self.registers.c)
                    }
                    PrefixTarget::B => {
                        self.registers.b = self.rotate_left_through_carry_set_zero(self.registers.b)
                    }
                    PrefixTarget::D => {
                        self.registers.d = self.rotate_left_through_carry_set_zero(self.registers.d)
                    }
                    PrefixTarget::E => {
                        self.registers.e = self.rotate_left_through_carry_set_zero(self.registers.e)
                    }
                    PrefixTarget::H => {
                        self.registers.h = self.rotate_left_through_carry_set_zero(self.registers.h)
                    }
                    PrefixTarget::L => {
                        self.registers.l = self.rotate_left_through_carry_set_zero(self.registers.l)
                    }
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.rotate_left_through_carry_set_zero(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RLA => {
                self.registers.a = self.rotate_left_through_carry_retain_zero(self.registers.a);
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RLC(register) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a = self.rotate_left_set_zero(self.registers.a)
                    }
                    PrefixTarget::B => {
                        self.registers.b = self.rotate_left_set_zero(self.registers.b)
                    }
                    PrefixTarget::C => {
                        self.registers.c = self.rotate_left_set_zero(self.registers.c)
                    }
                    PrefixTarget::D => {
                        self.registers.d = self.rotate_left_set_zero(self.registers.d)
                    }
                    PrefixTarget::E => {
                        self.registers.e = self.rotate_left_set_zero(self.registers.e)
                    }
                    PrefixTarget::H => {
                        self.registers.h = self.rotate_left_set_zero(self.registers.h)
                    }
                    PrefixTarget::L => {
                        self.registers.l = self.rotate_left_set_zero(self.registers.l)
                    }
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.rotate_left_set_zero(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RLCA => {
                self.registers.a = self.rotate_left_retain_zero(self.registers.a);
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RR(register) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a =
                            self.rotate_right_through_carry_set_zero(self.registers.a)
                    }
                    PrefixTarget::B => {
                        self.registers.b =
                            self.rotate_right_through_carry_set_zero(self.registers.b)
                    }
                    PrefixTarget::C => {
                        self.registers.c =
                            self.rotate_right_through_carry_set_zero(self.registers.c)
                    }
                    PrefixTarget::D => {
                        self.registers.d =
                            self.rotate_right_through_carry_set_zero(self.registers.d)
                    }
                    PrefixTarget::E => {
                        self.registers.e =
                            self.rotate_right_through_carry_set_zero(self.registers.e)
                    }
                    PrefixTarget::H => {
                        self.registers.h =
                            self.rotate_right_through_carry_set_zero(self.registers.h)
                    }
                    PrefixTarget::L => {
                        self.registers.l =
                            self.rotate_right_through_carry_set_zero(self.registers.l)
                    }
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.rotate_right_through_carry_set_zero(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RRA => {
                self.registers.a = self.rotate_right_through_carry_retain_zero(self.registers.a);
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RRC(register) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a = self.rotate_right_set_zero(self.registers.a)
                    }
                    PrefixTarget::B => {
                        self.registers.b = self.rotate_right_set_zero(self.registers.b)
                    }
                    PrefixTarget::C => {
                        self.registers.c = self.rotate_right_set_zero(self.registers.c)
                    }
                    PrefixTarget::D => {
                        self.registers.d = self.rotate_right_set_zero(self.registers.d)
                    }
                    PrefixTarget::E => {
                        self.registers.e = self.rotate_right_set_zero(self.registers.e)
                    }
                    PrefixTarget::H => {
                        self.registers.h = self.rotate_right_set_zero(self.registers.h)
                    }
                    PrefixTarget::L => {
                        self.registers.l = self.rotate_right_set_zero(self.registers.l)
                    }
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.rotate_right_set_zero(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::RRCA => {
                self.registers.a = self.rotate_right_retain_zero(self.registers.a);
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::RST(location) => {
                self.rst();
                (location.to_hex(), 16)
            }
            Instruction::SBC(target) => {
                match target {
                    ArithmeticTarget::A => self.sbc(self.registers.a),
                    ArithmeticTarget::B => self.sbc(self.registers.b),
                    ArithmeticTarget::C => self.sbc(self.registers.c),
                    ArithmeticTarget::D => self.sbc(self.registers.d),
                    ArithmeticTarget::E => self.sbc(self.registers.e),
                    ArithmeticTarget::H => self.sbc(self.registers.h),
                    ArithmeticTarget::L => self.sbc(self.registers.l),
                    ArithmeticTarget::HL => self.sbc(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.sbc(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::SCF => {
                self.registers.f.n = false;
                self.registers.f.h = false;
                self.registers.f.c = true;
                (self.pc.wrapping_add(1), 4)
            }
            Instruction::SET(register, bit_position) => {
                match register {
                    PrefixTarget::A => {
                        self.registers.a = self.set_bit(self.registers.a, bit_position)
                    }
                    PrefixTarget::B => {
                        self.registers.b = self.set_bit(self.registers.b, bit_position)
                    }
                    PrefixTarget::C => {
                        self.registers.c = self.set_bit(self.registers.c, bit_position)
                    }
                    PrefixTarget::D => {
                        self.registers.d = self.set_bit(self.registers.d, bit_position)
                    }
                    PrefixTarget::E => {
                        self.registers.e = self.set_bit(self.registers.e, bit_position)
                    }
                    PrefixTarget::H => {
                        self.registers.h = self.set_bit(self.registers.h, bit_position)
                    }
                    PrefixTarget::L => {
                        self.registers.l = self.set_bit(self.registers.l, bit_position)
                    }
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.set_bit(value, bit_position);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SLA(target) => {
                match target {
                    PrefixTarget::A => self.registers.a = self.sla(self.registers.a),
                    PrefixTarget::B => self.registers.b = self.sla(self.registers.b),
                    PrefixTarget::C => self.registers.c = self.sla(self.registers.c),
                    PrefixTarget::D => self.registers.d = self.sla(self.registers.d),
                    PrefixTarget::E => self.registers.e = self.sla(self.registers.e),
                    PrefixTarget::H => self.registers.h = self.sla(self.registers.h),
                    PrefixTarget::L => self.registers.l = self.sla(self.registers.l),
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.sla(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match target {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SRA(target) => {
                match target {
                    PrefixTarget::A => self.registers.a = self.sra(self.registers.a),
                    PrefixTarget::B => self.registers.b = self.sra(self.registers.b),
                    PrefixTarget::C => self.registers.c = self.sra(self.registers.c),
                    PrefixTarget::D => self.registers.d = self.sra(self.registers.d),
                    PrefixTarget::E => self.registers.e = self.sra(self.registers.e),
                    PrefixTarget::H => self.registers.h = self.sra(self.registers.h),
                    PrefixTarget::L => self.registers.l = self.sra(self.registers.l),
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.sra(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match target {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SRL(target) => {
                match target {
                    PrefixTarget::A => self.registers.a = self.srl(self.registers.a),
                    PrefixTarget::B => self.registers.b = self.srl(self.registers.b),
                    PrefixTarget::C => self.registers.c = self.srl(self.registers.c),
                    PrefixTarget::D => self.registers.d = self.srl(self.registers.d),
                    PrefixTarget::E => self.registers.e = self.srl(self.registers.e),
                    PrefixTarget::H => self.registers.h = self.srl(self.registers.h),
                    PrefixTarget::L => self.registers.l = self.srl(self.registers.l),
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.srl(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match target {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::SUB(target) => {
                match target {
                    ArithmeticTarget::A => self.sub(self.registers.a),
                    ArithmeticTarget::B => self.sub(self.registers.b),
                    ArithmeticTarget::C => self.sub(self.registers.c),
                    ArithmeticTarget::D => self.sub(self.registers.d),
                    ArithmeticTarget::E => self.sub(self.registers.e),
                    ArithmeticTarget::H => self.sub(self.registers.h),
                    ArithmeticTarget::L => self.sub(self.registers.l),
                    ArithmeticTarget::HL => self.sub(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.sub(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
            Instruction::SWAP(register) => {
                match register {
                    PrefixTarget::A => self.registers.a = self.swap(self.registers.a),
                    PrefixTarget::B => self.registers.b = self.swap(self.registers.b),
                    PrefixTarget::C => self.registers.c = self.swap(self.registers.c),
                    PrefixTarget::D => self.registers.d = self.swap(self.registers.d),
                    PrefixTarget::E => self.registers.e = self.swap(self.registers.e),
                    PrefixTarget::H => self.registers.h = self.swap(self.registers.h),
                    PrefixTarget::L => self.registers.l = self.swap(self.registers.l),
                    PrefixTarget::HL => {
                        let hl = self.registers.get_hl();
                        let value = self.mem.read_byte(hl);
                        let result = self.swap(value);
                        self.mem.write_byte(hl, result);
                    }
                }
                let cycles = match register {
                    PrefixTarget::HL => 16,
                    _ => 8,
                };
                (self.pc.wrapping_add(2), cycles)
            }
            Instruction::XOR(target) => {
                match target {
                    ArithmeticTarget::A => self.xor(self.registers.a),
                    ArithmeticTarget::B => self.xor(self.registers.b),
                    ArithmeticTarget::C => self.xor(self.registers.c),
                    ArithmeticTarget::D => self.xor(self.registers.d),
                    ArithmeticTarget::E => self.xor(self.registers.e),
                    ArithmeticTarget::H => self.xor(self.registers.h),
                    ArithmeticTarget::L => self.xor(self.registers.l),
                    ArithmeticTarget::HL => self.xor(self.mem.read_byte(self.registers.get_hl())),
                    ArithmeticTarget::D8 => self.xor(self.read_next_byte()),
                }
                match target {
                    ArithmeticTarget::D8 => (self.pc.wrapping_add(2), 8),
                    ArithmeticTarget::HL => (self.pc.wrapping_add(1), 8),
                    _ => (self.pc.wrapping_add(1), 4),
                }
            }
        }
    }

    fn push(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.mem.write_byte(self.sp, ((value & 0xFF00) >> 8) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.mem.write_byte(self.sp, (value & 0xFF) as u8);
    }

    fn pop(&mut self) -> u16 {
        let lsb = self.mem.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let msb = self.mem.read_byte(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (msb << 8) | lsb
    }

    fn read_next_byte(&self) -> u8 {
        self.mem.read_byte(self.pc + 1)
    }

    fn read_next_word(&self) -> u16 {
        (self.mem.read_byte(self.pc + 2) as u16) << 8 | self.mem.read_byte(self.pc + 1) as u16
    }

    fn adc(&mut self, value: u8) {
        let additional_carry = if self.registers.f.c { 1 } else { 0 };
        let (result, overflow) = self.registers.a.overflowing_add(value);
        let (result2, overflow2) = result.overflowing_add(additional_carry);
        self.registers.f.z = result2 == 0;
        self.registers.f.n = false;
        self.registers.f.h = ((self.registers.a & 0xF) + (value & 0xF) + additional_carry) > 0xF;
        self.registers.f.c = overflow || overflow2;
        self.registers.a = result2;
    }

    fn add(&mut self, value: u8) {
        let (result, overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.z = result == 0;
        self.registers.f.n = false;
        self.registers.f.h = (self.registers.a & 0xF) + (value & 0xF) > 0xF;
        self.registers.f.c = overflow;
        self.registers.a = result;
    }

    fn add_hl(&mut self, value: u16) {
        let hl = self.registers.get_hl();
        let (result, overflow) = hl.overflowing_add(value);
        self.registers.f.n = false;
        self.registers.f.h = (hl & 0xFFF) + (value & 0xFFF) > 0xFFF;
        self.registers.f.c = overflow;
        self.registers.set_hl(result);
    }

    fn and(&mut self, value: u8) {
        self.registers.a &= value;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
        self.registers.f.c = false;
    }

    fn test_bit(&mut self, value: u8, bit_position: BitPosition) {
        let bit_position: u8 = bit_position.into();
        let result = (value >> bit_position) & 0b1;
        self.registers.f.z = result == 0;
        self.registers.f.n = false;
        self.registers.f.h = true;
    }

    fn reset_bit(&mut self, value: u8, bit_position: BitPosition) -> u8 {
        let bit_position: u8 = u8::from(bit_position);
        value & !(1 << bit_position)
    }

    fn set_bit(&mut self, value: u8, bit_position: BitPosition) -> u8 {
        let bit_position: u8 = bit_position.into();
        value | (1 << bit_position)
    }

    fn swap(&mut self, value: u8) -> u8 {
        let new_value = ((value & 0xF) << 4) | ((value & 0xF0) >> 4);
        self.registers.f.z = new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
        return new_value;
    }

    fn call(&mut self, condition: bool) -> (u16, u8) {
        let next_pc = self.pc.wrapping_add(3);
        if condition {
            self.push(next_pc);
            (self.read_next_word(), 24)
        } else {
            (next_pc, 12)
        }
    }

    fn cp(&mut self, value: u8) {
        let (result, overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.z = result == 0;
        self.registers.f.n = true;
        self.registers.f.h = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.f.c = value > self.registers.a;
    }

    fn dec(&mut self, value: u8) -> u8 {
        let (result, overflow) = value.overflowing_sub(1);
        self.registers.f.z = result == 0;
        self.registers.f.n = true;
        self.registers.f.h = (value & 0xF) == 0;
        return result;
    }

    fn dec_16bit(&mut self, value: u16) -> u16 {
        let (result, overflow) = value.overflowing_sub(1);
        return result;
    }

    fn inc(&mut self, value: u8) -> u8 {
        let result = value.wrapping_add(1);
        self.registers.f.z = result == 0;
        self.registers.f.n = false;
        self.registers.f.h = value & 0xF == 0xF;
        result
    }

    fn inc_16bit(&mut self, value: u16) -> u16 {
        let (result, overflow) = value.overflowing_add(1);
        return result;
    }

    fn jump(&mut self, condition: bool) -> (u16, u8) {
        if condition {
            (self.read_next_word(), 16)
        } else {
            (self.pc.wrapping_add(3), 12)
        }
    }

    fn jump_relative(&mut self, condition: bool) -> (u16, u8) {
        let next_byte = self.pc.wrapping_add(2);
        if condition {
            let offset = self.read_next_byte() as i8;
            let pc = if offset >= 0 {
                next_byte.wrapping_add(offset as u16)
            } else {
                next_byte.wrapping_sub(offset.abs() as u16)
            };
            (pc, 12)
        } else {
            (next_byte, 8)
        }
    }

    fn or(&mut self, value: u8) {
        self.registers.a |= value;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }

    fn ret(&mut self, condition: bool) -> u16 {
        if condition {
            self.pop()
        } else {
            self.pc.wrapping_add(1)
        }
    }

    fn rotate_left_through_carry_retain_zero(&mut self, value: u8) -> u8 {
        self.rotate_left_through_carry(value, false)
    }

    fn rotate_left_through_carry_set_zero(&mut self, value: u8) -> u8 {
        self.rotate_left_through_carry(value, true)
    }

    fn rotate_left_through_carry(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry = if self.registers.f.c { 1 } else { 0 };
        let new_value = (value << 1) | carry;
        self.registers.f.z = set_zero && new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0x80 != 0;
        return new_value;
    }

    fn rotate_left_retain_zero(&mut self, value: u8) -> u8 {
        self.rotate_left(value, false)
    }

    fn rotate_left_set_zero(&mut self, value: u8) -> u8 {
        self.rotate_left(value, true)
    }

    fn rotate_left(&mut self, value: u8, set_zero: bool) -> u8 {
        // let carry = (value & 0x80) >> 7;
        let new_value = value.rotate_left(1);
        self.registers.f.z = set_zero && new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0x80 != 0;
        return new_value;
    }

    fn rotate_right_through_carry_retain_zero(&mut self, value: u8) -> u8 {
        self.rotate_right_through_carry(value, false)
    }

    fn rotate_right_through_carry_set_zero(&mut self, value: u8) -> u8 {
        self.rotate_right_through_carry(value, true)
    }

    fn rotate_right_through_carry(&mut self, value: u8, set_zero: bool) -> u8 {
        let carry = if self.registers.f.c { 1 } else { 0 } << 7;
        let new_value = (value >> 1) | carry;
        self.registers.f.z = set_zero && new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0b1 == 0b1;
        return new_value;
    }

    fn rotate_right_retain_zero(&mut self, value: u8) -> u8 {
        self.rotate_right(value, false)
    }

    fn rotate_right_set_zero(&mut self, value: u8) -> u8 {
        self.rotate_right(value, true)
    }

    fn rotate_right(&mut self, value: u8, set_zero: bool) -> u8 {
        // let carry = (value & 0x80) >> 7;
        let new_value = value.rotate_right(1);
        self.registers.f.z = set_zero && new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0b1 == 0b1;
        return new_value;
    }

    fn rst(&mut self) {
        self.push(self.pc.wrapping_add(1));
    }

    fn sbc(&mut self, value: u8) {
        let additional_carry = if self.registers.f.c { 1 } else { 0 };
        let (result, overflow) = self.registers.a.overflowing_sub(value);
        let (result2, overflow2) = result.overflowing_sub(additional_carry);
        self.registers.f.z = result2 == 0;
        self.registers.f.n = true;
        self.registers.f.h = (self.registers.a & 0xF) < (value & 0xF) + additional_carry;
        self.registers.f.c = overflow || overflow2;
        self.registers.a = result2;
    }

    fn sla(&mut self, value: u8) -> u8 {
        let new_value = value << 1;
        self.registers.f.z = new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0x80 != 0;
        return new_value;
    }

    fn sra(&mut self, value: u8) -> u8 {
        let msb = value & 0x80;
        let new_value = msb | (value >> 1);
        self.registers.f.z = new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0x1 != 0;
        return new_value;
    }

    fn srl(&mut self, value: u8) -> u8 {
        let new_value = value >> 1;
        self.registers.f.z = new_value == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = value & 0x1 != 0;
        return new_value;
    }

    fn sub(&mut self, value: u8) {
        let (result, overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.z = result == 0;
        self.registers.f.n = true;
        self.registers.f.h = (self.registers.a & 0xF) < (value & 0xF);
        self.registers.f.c = value > self.registers.a;
        self.registers.a = result;
    }

    fn xor(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.f.z = self.registers.a == 0;
        self.registers.f.n = false;
        self.registers.f.h = false;
        self.registers.f.c = false;
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     macro_rules! test_instruction {
//         ( $instruction:expr, $( $($register:ident).* => $value:expr ),* ) => {
//             {
//                 let mut cpu = CPU::new();
//                 $(
//                     cpu.registers$(.$register)* = $value;
//                 )*
//                 cpu.execute($instruction);
//                 cpu
//             }
//         };
//     }

//     macro_rules! check_flags {
//         ( $cpu:ident, zero => $zero:ident, subtract => $subtract:ident, half_carry => $half_carry:ident, carry => $carry:ident ) => {{
//             let flags = $cpu.registers.f;
//             assert_eq!(flags.z, $zero);
//             assert_eq!(flags.n, $subtract);
//             assert_eq!(flags.h, $half_carry);
//             assert_eq!(flags.c, $carry);
//         }};
//     }

//     #[test]
//     fn test_adc() {
//         let cpu = test_instruction!(Instruction::ADC(ArithmeticTarget::A), a => 0x7, f.c => true);
//         assert_eq!(cpu.registers.a, 0xf);
//     }

//     #[test]
//     fn test_add() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.registers.a = 2;
//         cpu.execute(Instruction::ADD(ArithmeticTarget::B));
//         assert_eq!(cpu.registers.a, 3);
//     }

//     #[test]
//     fn test_and() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.registers.a = 2;
//         cpu.execute(Instruction::AND(ArithmeticTarget::B));
//         assert_eq!(cpu.registers.a, 0);
//     }

//     #[test]
//     fn test_cp() {
//         let cpu = test_instruction!(Instruction::CP(ArithmeticTarget::A), a => 0x7);
//         assert_eq!(cpu.registers.a, 0x7);
//         check_flags!(cpu, zero => true, subtract => true, half_carry => false, carry => false);
//     }

//     #[test]
//     fn test_sub() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.registers.a = 2;
//         cpu.execute(Instruction::SUB(ArithmeticTarget::B));
//         assert_eq!(cpu.registers.a, 1);
//     }

//     #[test]
//     fn test_dec() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.execute(Instruction::DEC(IncDecTarget::B));
//         assert_eq!(cpu.registers.b, 0);
//     }

//     #[test]
//     fn test_inc() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.execute(Instruction::INC(IncDecTarget::B));
//         assert_eq!(cpu.registers.b, 2);
//     }

//     #[test]
//     fn test_or() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.registers.a = 2;
//         cpu.execute(Instruction::OR(ArithmeticTarget::B));
//         assert_eq!(cpu.registers.a, 3);
//     }

//     #[test]
//     fn test_xor() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 1;
//         cpu.registers.a = 2;
//         cpu.execute(Instruction::XOR(ArithmeticTarget::B));
//         assert_eq!(cpu.registers.a, 3);
//     }

//     #[test]
//     fn execute_rl() {
//         let cpu = test_instruction!(Instruction::RL(PrefixTarget::A), a => 0b1011_0101);

//         assert_eq!(cpu.registers.a, 0b0110_1010);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);

//         let cpu =
//             test_instruction!(Instruction::RL(PrefixTarget::A), a => 0b1011_0101, f.c => true);

//         assert_eq!(cpu.registers.a, 0b0110_1011);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_rla_8bit() {
//         let cpu = test_instruction!(Instruction::RLA, a => 0x80);

//         assert_eq!(cpu.registers.a, 0x0);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_rlca_8bit() {
//         let cpu = test_instruction!(Instruction::RLCA, a => 0x80, f.c => true);

//         assert_eq!(cpu.registers.a, 0x1);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_rr() {
//         let cpu = test_instruction!(Instruction::RR(PrefixTarget::A), a => 0b1011_0101);

//         assert_eq!(cpu.registers.a, 0b0101_1010);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);

//         let cpu =
//             test_instruction!(Instruction::RR(PrefixTarget::A), a => 0b1011_0101, f.c => true);

//         assert_eq!(cpu.registers.a, 0b1101_1010);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_rra_8bit() {
//         let cpu = test_instruction!(Instruction::RRA, a => 0b1);

//         assert_eq!(cpu.registers.a, 0x0);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_rrca_8bit() {
//         let cpu = test_instruction!(Instruction::RRCA, a => 0b1, f.c => true);

//         assert_eq!(cpu.registers.a, 0x80);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => true);
//     }

//     #[test]
//     fn execute_load_byte() {
//         let cpu = test_instruction!(Instruction::LD(LoadType::Byte(LoadByteTarget::D, LoadByteSource::B)), b => 0x4);
//         assert_eq!(cpu.registers.d, 0x4);
//         assert_eq!(cpu.registers.b, 0x4);
//     }

//     // INC
//     #[test]
//     fn execute_inc_8bit_non_overflow() {
//         let cpu = test_instruction!(Instruction::INC(IncDecTarget::A), a => 0x7);

//         assert_eq!(cpu.registers.a, 0x8);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     #[test]
//     fn execute_inc_8bit_half_carry() {
//         let cpu = test_instruction!(Instruction::INC(IncDecTarget::A), a => 0xF);

//         assert_eq!(cpu.registers.a, 0x10);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => true, carry => false);
//     }

//     #[test]
//     fn execute_inc_8bit_overflow() {
//         let cpu = test_instruction!(Instruction::INC(IncDecTarget::A), a => 0xFF);

//         assert_eq!(cpu.registers.a, 0x0);
//         check_flags!(cpu, zero => true, subtract => false, half_carry => true, carry => false);
//     }

//     #[test]
//     fn execute_inc_16bit_byte_overflow() {
//         let instruction = Instruction::INC(IncDecTarget::HL);
//         let mut cpu = CPU::new();
//         cpu.registers.set_hl(0xFF);
//         cpu.execute(instruction);

//         assert_eq!(cpu.registers.get_hl(), 0x0100);
//         assert_eq!(cpu.registers.h, 0x01);
//         assert_eq!(cpu.registers.l, 0x00);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     #[test]
//     fn execute_inc_16bit_overflow() {
//         let instruction = Instruction::INC(IncDecTarget::BC);
//         let mut cpu = CPU::new();
//         cpu.registers.set_bc(0xFFFF);
//         cpu.execute(instruction);

//         assert_eq!(cpu.registers.get_bc(), 0x0);
//         assert_eq!(cpu.registers.b, 0x00);
//         assert_eq!(cpu.registers.c, 0x00);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     // OR
//     #[test]
//     fn execute_or_8bit() {
//         let cpu = test_instruction!(Instruction::OR(ArithmeticTarget::A), a => 0x7);

//         assert_eq!(cpu.registers.a, 0x7);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     #[test]
//     fn execute_or_8bit_with_zero() {
//         let cpu = test_instruction!(Instruction::OR(ArithmeticTarget::B), a => 0x8);

//         assert_eq!(cpu.registers.a, 0x8);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     // SET
//     #[test]
//     fn execute_set_8bit() {
//         let cpu =
//             test_instruction!(Instruction::SET(PrefixTarget::A, BitPosition::B2), a => 0b1011_0100);

//         assert_eq!(cpu.registers.a, 0b1011_0100);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);

//         let cpu =
//             test_instruction!(Instruction::SET(PrefixTarget::A, BitPosition::B1), a => 0b1011_0100);
//         assert_eq!(cpu.registers.a, 0b1011_0110);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     // RES
//     #[test]
//     fn execute_res_8bit() {
//         let cpu =
//             test_instruction!(Instruction::RES(PrefixTarget::A, BitPosition::B2), a => 0b1011_0100);

//         assert_eq!(cpu.registers.a, 0b1011_0000);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);

//         let cpu =
//             test_instruction!(Instruction::RES(PrefixTarget::A, BitPosition::B1), a => 0b1011_0100);
//         assert_eq!(cpu.registers.a, 0b1011_0100);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     // JP
//     #[test]
//     fn execute_jp() {
//         let mut cpu = CPU::new();
//         cpu.pc = 0xF8;
//         cpu.mem.write_byte(0xF9, 0xFC);
//         cpu.mem.write_byte(0xFA, 0x02);
//         let (next_pc, _) = cpu.execute(Instruction::JP(JumpCondition::Always));

//         assert_eq!(next_pc, 0x02FC);

//         let (next_pc, _) = cpu.execute(Instruction::JP(JumpCondition::Carry));

//         assert_eq!(next_pc, 0xFB);
//     }

//     // JR
//     #[test]
//     fn execute_jr() {
//         let mut cpu = CPU::new();
//         cpu.pc = 0xF8;
//         cpu.mem.write_byte(0xF9, 0x4);
//         let (next_pc, _) = cpu.execute(Instruction::JR(JumpCondition::Always));

//         assert_eq!(next_pc, 0xFE);

//         cpu.mem.write_byte(0xF9, 0xFC); // == -4
//         let (next_pc, _) = cpu.execute(Instruction::JR(JumpCondition::Always));
//         assert_eq!(next_pc, 0xF6);
//     }

//     // LD a, (??)
//     #[test]
//     fn execute_ld_a_indirect() {
//         let mut cpu = CPU::new();
//         cpu.registers.set_bc(0xF9);
//         cpu.mem.write_byte(0xF9, 0x4);
//         cpu.execute(Instruction::LD(LoadType::AFromIndirect(IndirectTarget::BC)));

//         assert_eq!(cpu.registers.a, 0x04);

//         cpu.registers.set_hl(0xA1);
//         cpu.mem.write_byte(0xA1, 0x9);
//         cpu.execute(Instruction::LD(LoadType::AFromIndirect(
//             IndirectTarget::HLI,
//         )));

//         assert_eq!(cpu.registers.a, 0x09);
//         assert_eq!(cpu.registers.get_hl(), 0xA2);
//     }

//     // LD ?, ?
//     #[test]
//     fn execute_ld_byte() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 0x4;
//         cpu.execute(Instruction::LD(LoadType::Byte(
//             LoadByteTarget::D,
//             LoadByteSource::B,
//         )));

//         assert_eq!(cpu.registers.b, 0x4);
//         assert_eq!(cpu.registers.d, 0x4);
//     }

//     // LD [FF00 + n8], A
//     // #[test]
//     // fn execute_ld_io() {
//     //     let mut cpu = CPU::new();
//     //     cpu.registers.a = 0x4;
//     //     cpu.mem.write_byte(0x01, 0xFF);
//     //     cpu.execute(Instruction::LD(LoadType::AFromByteAddress));

//     //     assert_eq!(cpu.mem.read_byte(0xFFFF), 0x4);
//     // }

//     // #[test]
//     // fn execute_load_word() {
//     //     let cpu = test_instruction!(Instruction::LD(LoadType::Word(LoadWordTarget::BC)), pc => 0x4);
//     //     assert_eq!(cpu.registers.get_bc(), 0x4);
//     // }

//     // SWAP
//     #[test]
//     fn execute_swap() {
//         let cpu = test_instruction!(Instruction::SWAP(PrefixTarget::A), a => 0b1011_0101);

//         assert_eq!(cpu.registers.a, 0b0101_1011);
//         check_flags!(cpu, zero => false, subtract => false, half_carry => false, carry => false);
//     }

//     // PUSH/POP
//     #[test]
//     fn execute_push_pop() {
//         let mut cpu = CPU::new();
//         cpu.registers.b = 0x4;
//         cpu.registers.c = 0x89;
//         cpu.sp = 0x10;
//         cpu.execute(Instruction::PUSH(StackTarget::BC));

//         assert_eq!(cpu.mem.read_byte(0xF), 0x04);
//         assert_eq!(cpu.mem.read_byte(0xE), 0x89);
//         assert_eq!(cpu.sp, 0xE);

//         cpu.execute(Instruction::POP(StackTarget::DE));

//         assert_eq!(cpu.registers.d, 0x04);
//         assert_eq!(cpu.registers.e, 0x89);
//     }

//     // Step
//     #[test]
//     fn test_step() {
//         let mut cpu = CPU::new();
//         cpu.mem.write_byte(0, 0x23); //INC(HL)
//         cpu.mem.write_byte(1, 0xB5); //OR(L)
//         cpu.mem.write_byte(2, 0xCB); //PREFIX
//         cpu.mem.write_byte(3, 0xe8); //SET(B, 5)
//         for _ in 0..3 {
//             cpu.step();
//         }

//         assert_eq!(cpu.registers.h, 0b0);
//         assert_eq!(cpu.registers.l, 0b1);
//         assert_eq!(cpu.registers.a, 0b1);
//         assert_eq!(cpu.registers.b, 0b0010_0000);
//     }
// }
