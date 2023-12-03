use super::peripherals::{InterruptMask, InterruptSource, Peripherals};

mod decoder;
mod pc_reader;
mod registers;

use decoder::{
    AddToSpDestination, ArithmeticLogic, BitOp, Condition, Destination, IncDecDirection,
    Instruction, LoadMemoryDirection, LoadMemoryLocation, Operand8, ResetSlot, RotateCarry,
    RotateDirection,
};
use pc_reader::PcReader;
use registers::Registers;

pub struct Cpu {
    cycle: u64,
    registers: Registers,
    halted: bool,
    interrupt_enable: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            cycle: 0,
            registers: Registers::new(),
            halted: false,
            interrupt_enable: false,
        }
    }

    pub(crate) fn cycle(&self) -> u64 {
        self.cycle
    }

    pub(crate) fn run(&mut self, peripherals: &mut Peripherals, cycles: u64) {
        let end_cycle = self.cycle + cycles;

        while self.cycle < end_cycle {
            self.step(peripherals)
        }
    }

    pub(crate) fn step(&mut self, peripherals: &mut Peripherals) {
        let mut pc = self.registers.pc;

        if self.interrupt_enable {
            let mut reg_if: InterruptMask = peripherals.read(self.cycle, 0xff0f).into();
            let reg_ie: InterruptMask = peripherals.read(self.cycle, 0xffff).into();

            let pending = reg_if & reg_ie;

            if let Some(interrupt) = pending.highest_priority() {
                let sp = self.registers.sp.wrapping_sub(2);
                peripherals.write_u16(self.cycle, sp, pc);
                self.registers.sp = sp;

                reg_if.clear(interrupt);
                peripherals.write(self.cycle, 0xff0f, reg_if.into());
                self.halted = false;
                self.interrupt_enable = false;
                pc = interrupt.vector_address();
            }
        }

        if self.halted {
            self.cycle = peripherals.next_pending(self.cycle);
            return;
        }

        let inst = {
            let mut reader = PcReader::new(self.cycle, &mut pc, peripherals);
            Instruction::from_pc_reader(&mut reader)
        };

        let cycles = match inst {
            Instruction::Add16(operand) => {
                let hl = self.registers.hl();
                let other = self.registers.read_double(operand);

                let (res, carry) = hl.overflowing_add(other);

                self.registers.set_hl(res);
                self.registers.set_carry(carry);

                let half_hl = hl & 0x000f;
                let half_other = other & 0x000f;

                let half_carry = (half_hl + half_other) > 0xf;

                self.registers.set_bcd_n(false);
                self.registers.set_bcd_h(half_carry);

                8
            }
            Instruction::AddToSp(destination, immediate) => {
                let (res, carry) = self.registers.sp.overflowing_add_signed(immediate as i16);

                self.registers.set_zero(false);
                self.registers.set_carry(carry);
                self.registers.set_bcd_n(false);
                self.registers
                    .set_bcd_h((res ^ self.registers.sp) & 0b0001_0000 != 0);

                match destination {
                    AddToSpDestination::Hl => self.registers.set_hl(res),
                    AddToSpDestination::Sp => self.registers.sp = res,
                }

                16
            }
            Instruction::ArithmeticLogic8(operation, operand) => {
                let c = self.registers.carry();
                let a = self.registers.a;
                let (load_cycles, other) = self.load_operand8(peripherals, operand);

                let half_a = a & 0x0f;
                let half_other = other & 0x0f;

                let (flag_n, flag_h, flag_c, val) = match operation {
                    ArithmeticLogic::Add => {
                        let (_, half_carry) = (half_a | 0xf0).overflowing_add(half_other);
                        let (val, carry) = a.overflowing_add(other);

                        (false, half_carry, carry, val)
                    }
                    ArithmeticLogic::Adc => {
                        let (tmp, half_carry1) = (half_a | 0xf0).overflowing_add(c as u8);
                        let (_, half_carry2) = tmp.overflowing_add(half_other);

                        let (tmp, carry1) = a.overflowing_add(c as u8);
                        let (val, carry2) = tmp.overflowing_add(other);

                        (false, half_carry1 || half_carry2, carry1 || carry2, val)
                    }
                    ArithmeticLogic::Sub | ArithmeticLogic::Cp => {
                        let (_, half_carry) = half_a.overflowing_sub(half_other);
                        let (val, carry) = a.overflowing_sub(other);

                        (true, half_carry, carry, val)
                    }
                    ArithmeticLogic::Sbc => {
                        let (tmp, half_carry1) = half_a.overflowing_sub(c as u8);
                        let (_, half_carry2) = tmp.overflowing_sub(half_other);

                        let (tmp, carry1) = a.overflowing_sub(c as u8);
                        let (val, carry2) = tmp.overflowing_sub(other);

                        (true, half_carry1 || half_carry2, carry1 || carry2, val)
                    }
                    ArithmeticLogic::And => (false, true, false, a & other),
                    ArithmeticLogic::Xor => (false, false, false, a ^ other),
                    ArithmeticLogic::Or => (false, false, false, a | other),
                };

                self.registers.set_zero(val == 0);
                self.registers.set_bcd_n(flag_n);
                self.registers.set_bcd_h(flag_h);
                self.registers.set_carry(flag_c);

                if !matches!(operation, ArithmeticLogic::Cp) {
                    self.registers.a = val;
                }

                load_cycles + 4
            }
            Instruction::BitOp(operation, operand) => {
                let (load_cycles, input) = self.load_operand8(peripherals, operand);

                let (flag_z, flag_n, flag_h, flag_c, val) = match operation {
                    BitOp::Rotate(RotateDirection::Left, RotateCarry::Through) => {
                        // RLC
                        let c = (input & 0b1000_0000) != 0;
                        let val = (input << 1) | (input >> 7);
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::Rotate(RotateDirection::Right, RotateCarry::Through) => {
                        // RRC
                        let c = (input & 0b0000_0001) != 0;
                        let val = (input >> 1) | (input << 7);
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::Rotate(RotateDirection::Left, RotateCarry::NotThrough) => {
                        // RL
                        let c = (input & 0b1000_0000) != 0;
                        let val = (input << 1) | (self.registers.carry() as u8);
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::Rotate(RotateDirection::Right, RotateCarry::NotThrough) => {
                        // RR
                        let c = (input & 0b0000_0001) != 0;
                        let val = (input >> 1) | (self.registers.carry() as u8) << 7;
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::ShiftArithmetic(RotateDirection::Left) => {
                        let c = (input & 0b1000_0000) != 0;
                        let val = input << 1;
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::ShiftArithmetic(RotateDirection::Right) => {
                        let c = (input & 0b0000_0001) != 0;
                        let val = input >> 1 | (input & 0b1000_0000);
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::SwapNibbles => {
                        let val = (input >> 4) | (input << 4);
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(false), Some(val))
                    }
                    BitOp::ShiftRightLogical => {
                        let c = (input & 0b0000_0001) != 0;
                        let val = input >> 1;
                        let z = val == 0;
                        (Some(z), Some(false), Some(false), Some(c), Some(val))
                    }
                    BitOp::Test(bit) => {
                        let z = input & bit.mask() == 0;
                        (Some(z), Some(false), Some(true), None, None)
                    }
                    BitOp::Clear(bit) => {
                        let val = input & !bit.mask();
                        (None, None, None, None, Some(val))
                    }
                    BitOp::Set(bit) => {
                        let val = input | bit.mask();
                        (None, None, None, None, Some(val))
                    }
                };

                let store_cycles = match val {
                    Some(val) => self.store_operand8(peripherals, operand, val),
                    None => 0,
                };

                if let Some(z) = flag_z {
                    self.registers.set_zero(z);
                }

                if let Some(n) = flag_n {
                    self.registers.set_bcd_n(n);
                }

                if let Some(h) = flag_h {
                    self.registers.set_bcd_h(h);
                }

                if let Some(c) = flag_c {
                    self.registers.set_carry(c);
                }

                load_cycles + store_cycles + 2
            }
            Instruction::Call(destination, condition) => {
                let (load_cycles, addr) = self.jump_destination(pc, destination);

                if self.check_condition(condition) {
                    let sp = self.registers.sp.wrapping_sub(2);
                    peripherals.write_u16(self.cycle, sp, pc);
                    self.registers.sp = sp;

                    pc = addr;
                    load_cycles + 4
                } else {
                    load_cycles
                }
            }
            Instruction::Ccf => {
                self.registers.set_carry(false);
                4
            }
            Instruction::Cpl => {
                self.registers.a ^= 0xff;
                self.registers.set_bcd_n(true);
                self.registers.set_bcd_h(true);
                4
            }
            Instruction::Ei => {
                self.interrupt_enable = true;
                4
            }
            Instruction::Daa => {
                let mut a = self.registers.a;

                if self.registers.bcd_n() {
                    if self.registers.carry() {
                        a = a.wrapping_sub(0x60);
                    }
                    if self.registers.bcd_h() {
                        a = a.wrapping_sub(0x06);
                    }
                } else {
                    if self.registers.carry() || a > 0x99 {
                        a = a.wrapping_add(0x60);
                        self.registers.set_carry(true);
                    }
                    if self.registers.bcd_h() || (a & 0x0f) > 0x09 {
                        a = a.wrapping_add(0x06);
                    }
                }

                self.registers.set_zero(a == 0);
                self.registers.set_bcd_h(false);
                self.registers.a = a;

                4
            }
            Instruction::Di => {
                self.interrupt_enable = false;
                4
            }
            Instruction::Halt => {
                self.halted = true;
                4
            }
            Instruction::IncDec8(direction, operand) => {
                let (load_cycles, value) = self.load_operand8(peripherals, operand);

                let (result, flag_n) = match direction {
                    IncDecDirection::Dec => (value.wrapping_sub(1), true),
                    IncDecDirection::Inc => (value.wrapping_add(1), false),
                };

                let store_cycles = self.store_operand8(peripherals, operand, result);

                self.registers.set_zero(result == 0);
                self.registers.set_bcd_n(flag_n);
                self.registers
                    .set_bcd_h(((value ^ result) & 0b0001_0000) != 0);

                load_cycles + store_cycles + 4
            }
            Instruction::IncDec16(direction, operand) => {
                let value = self.registers.read_double(operand);

                let result = match direction {
                    IncDecDirection::Dec => value.wrapping_sub(1),
                    IncDecDirection::Inc => value.wrapping_add(1),
                };

                self.registers.write_double(operand, result);

                8
            }
            Instruction::Jump(destination, condition) => {
                let (load_cycles, addr) = self.jump_destination(pc, destination);

                if self.check_condition(condition) {
                    pc = addr;
                    load_cycles + 4
                } else {
                    load_cycles
                }
            }
            Instruction::LoadHlToSp => {
                self.registers.sp = self.registers.hl();
                8
            }
            Instruction::LoadImm16(destination, immediate) => {
                self.registers.write_double(destination, immediate);
                12
            }
            Instruction::LoadMemory(direction, location) => {
                let (addr, cycles) = match location {
                    LoadMemoryLocation::Bc => (self.registers.bc(), 8),
                    LoadMemoryLocation::De => (self.registers.de(), 8),
                    LoadMemoryLocation::HlDec | LoadMemoryLocation::HlInc => {
                        (self.registers.hl(), 8)
                    }
                    LoadMemoryLocation::ZeroPageC => (0xff00 | self.registers.c as u16, 8),
                    LoadMemoryLocation::ZeroPageImm(immediate) => (0xff00 | immediate as u16, 12),
                    LoadMemoryLocation::Absolute(addr) => (addr, 16),
                };

                match direction {
                    LoadMemoryDirection::FromMemory => {
                        self.registers.a = peripherals.read(self.cycle, addr);
                    }
                    LoadMemoryDirection::ToMemory => {
                        peripherals.write(self.cycle, addr, self.registers.a);
                    }
                }

                match location {
                    LoadMemoryLocation::HlDec => {
                        self.registers.set_hl(addr.wrapping_sub(1));
                    }
                    LoadMemoryLocation::HlInc => {
                        self.registers.set_hl(addr.wrapping_add(1));
                    }
                    _ => {}
                }

                cycles
            }
            Instruction::LoadSimple(destination, source) => {
                let (load_cycles, value) = self.load_operand8(peripherals, source);
                let store_cycles = self.store_operand8(peripherals, destination, value);
                load_cycles + store_cycles + 4
            }
            Instruction::LoadSpToImm(addr) => {
                peripherals.write_u16(self.cycle, addr, self.registers.sp);
                20
            }
            Instruction::Nop => 4,
            Instruction::Pop(operand) => {
                let val = peripherals.read_u16(self.cycle, self.registers.sp);
                self.registers.write_double(operand, val);
                self.registers.sp = self.registers.sp.wrapping_add(2);

                12
            }
            Instruction::Push(operand) => {
                let val = self.registers.read_double(operand);
                let sp = self.registers.sp.wrapping_sub(2);
                peripherals.write_u16(self.cycle, sp, val);
                self.registers.sp = sp;

                16
            }
            Instruction::Reset(vector) => {
                let sp = self.registers.sp.wrapping_sub(2);
                peripherals.write_u16(self.cycle, sp, pc);
                self.registers.sp = sp;

                pc = match vector {
                    ResetSlot::Slot0 => 0x0000,
                    ResetSlot::Slot1 => 0x0008,
                    ResetSlot::Slot2 => 0x0010,
                    ResetSlot::Slot3 => 0x0018,
                    ResetSlot::Slot4 => 0x0020,
                    ResetSlot::Slot5 => 0x0028,
                    ResetSlot::Slot6 => 0x0030,
                    ResetSlot::Slot7 => 0x0038,
                };

                16
            }
            Instruction::Reti => {
                pc = peripherals.read_u16(self.cycle, self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(2);
                self.interrupt_enable = true;
                4
            }
            Instruction::Return(condition) => {
                let condition_penalty = match condition {
                    Condition::Always => 0,
                    _ => 4,
                };

                if self.check_condition(condition) {
                    pc = peripherals.read_u16(self.cycle, self.registers.sp);
                    self.registers.sp = self.registers.sp.wrapping_add(2);

                    condition_penalty + 16
                } else {
                    condition_penalty + 4
                }
            }
            Instruction::RotateA(direction, through) => {
                let a = self.registers.a;
                let c = self.registers.carry() as u8;

                let (val, carry) = match (direction, through) {
                    (RotateDirection::Left, RotateCarry::Through) => {
                        (a << 1 | a >> 7, a & 0b1000_0000 != 0)
                    }
                    (RotateDirection::Right, RotateCarry::Through) => {
                        (a >> 1 | a << 7, a & 0b0000_0001 != 0)
                    }
                    (RotateDirection::Left, RotateCarry::NotThrough) => {
                        (a << 1 | c, a & 0b1000_0000 != 0)
                    }
                    (RotateDirection::Right, RotateCarry::NotThrough) => {
                        (a >> 1 | c << 7, a & 0b0000_0001 != 0)
                    }
                };

                self.registers.a = val;
                self.registers.set_zero(false);
                self.registers.set_bcd_n(false);
                self.registers.set_bcd_h(false);
                self.registers.set_carry(carry);

                4
            }
            Instruction::Scf => {
                self.registers.set_carry(true);
                4
            }
            Instruction::Stop => {
                unimplemented!("STOP")
            }
            Instruction::Invalid(op) => {
                unimplemented!("Invalid instruction {op}")
            }
        };

        self.cycle += cycles;

        self.registers.pc = pc;
    }

    fn load_operand8(&self, peripherals: &Peripherals, operand: Operand8) -> (u64, u8) {
        match operand {
            Operand8::Register(register) => (0, self.registers.read(register)),
            Operand8::IndirectHl => {
                let addr = self.registers.hl();
                let val = peripherals.read(self.cycle, addr);
                (4, val)
            }
            Operand8::Immediate(immediate) => (4, immediate),
        }
    }

    fn store_operand8(
        &mut self,
        peripherals: &mut Peripherals,
        operand: Operand8,
        value: u8,
    ) -> u64 {
        match operand {
            Operand8::Register(register) => {
                self.registers.write(register, value);
                0
            }
            Operand8::IndirectHl => {
                let addr = self.registers.hl();
                peripherals.write(self.cycle, addr, value);
                4
            }
            Operand8::Immediate(_) => panic!("Tried to store to immediate _value_"),
        }
    }

    fn check_condition(&self, condition: Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::NonCarry => !self.registers.carry(),
            Condition::NonZero => !self.registers.zero(),
            Condition::Carry => self.registers.carry(),
            Condition::Zero => self.registers.zero(),
        }
    }

    fn jump_destination(&self, pc: u16, destination: Destination) -> (u64, u16) {
        match destination {
            Destination::Absolute(addr) => (12, addr),
            Destination::Relative(offset) => {
                let addr = pc.wrapping_add_signed(offset as i16);
                (8, addr)
            }
            Destination::Hl => {
                let addr = self.registers.hl();
                (0, addr)
            }
        }
    }
}
