use super::pc_reader::PcReader;

#[derive(Debug, Clone, Copy)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Debug, Clone, Copy)]
pub enum Operand8 {
    Register(Register),
    IndirectHl,
    Immediate(u8),
}

impl Operand8 {
    fn from_op_bits123(op: u8) -> Self {
        match op & 0b0000_0111 {
            0 => Self::Register(Register::B),
            1 => Self::Register(Register::C),
            2 => Self::Register(Register::D),
            3 => Self::Register(Register::E),
            4 => Self::Register(Register::H),
            5 => Self::Register(Register::L),
            6 => Self::IndirectHl,
            7 => Self::Register(Register::A),
            masked => panic!("Impossible 3 bit Register {masked}"),
        }
    }

    fn from_op_bits456(op: u8) -> Self {
        Self::from_op_bits123(op >> 3)
    }

    fn from_immediate(reader: &mut PcReader) -> Self {
        Self::Immediate(reader.read_u8())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Operand16 {
    Bc,
    De,
    Hl,
    Sp,
    Af,
}

impl Operand16 {
    fn from_op_bits56_sp(op: u8) -> Self {
        match op & 0b0011_0000 {
            0x00 => Self::Bc,
            0x10 => Self::De,
            0x20 => Self::Hl,
            0x30 => Self::Sp,
            masked => panic!("Impossible 16 bit operand {masked} op {op:02x}"),
        }
    }

    fn from_op_bits56_af(op: u8) -> Self {
        match op & 0b0011_0000 {
            0x00 => Self::Bc,
            0x10 => Self::De,
            0x20 => Self::Hl,
            0x30 => Self::Af,
            masked => panic!("Impossible 16 bit operand {masked} op {op:02x}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ArithmeticLogic {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Xor,
    Or,
    Cp,
}

impl ArithmeticLogic {
    fn from_op_bits456(op: u8) -> Self {
        match op & 0b0011_1000 {
            0x00 => Self::Add,
            0x08 => Self::Adc,
            0x10 => Self::Sub,
            0x18 => Self::Sbc,
            0x20 => Self::And,
            0x28 => Self::Xor,
            0x30 => Self::Or,
            0x38 => Self::Cp,
            masked => {
                panic!("Impossible ArithmeticLogic instruction {op:02x} (masked {masked:02x})")
            }
        }
    }
}

#[derive(Debug)]
pub enum IncDecDirection {
    Inc,
    Dec,
}

impl IncDecDirection {
    fn from_op_bit1(op: u8) -> Self {
        if op & 0b0000_0001 == 0 {
            Self::Inc
        } else {
            Self::Dec
        }
    }

    fn from_op_bit4(op: u8) -> Self {
        if op & 0b0000_1000 == 0 {
            Self::Inc
        } else {
            Self::Dec
        }
    }
}

#[derive(Debug)]
pub enum RotateDirection {
    Left,
    Right,
}

impl RotateDirection {
    fn from_op_bit4(op: u8) -> Self {
        if op & 0b0000_1000 == 0 {
            Self::Left
        } else {
            Self::Right
        }
    }
}

#[derive(Debug)]
pub enum RotateCarry {
    Through,
    NotThrough,
}

impl RotateCarry {
    fn from_op_bit5(op: u8) -> Self {
        if op & 0b0001_0000 == 0 {
            Self::Through
        } else {
            Self::NotThrough
        }
    }
}

#[derive(Debug)]
pub enum LoadMemoryDirection {
    ToMemory,
    FromMemory,
}

impl LoadMemoryDirection {
    fn from_op_bit4(op: u8) -> Self {
        if op & 0b0000_1000 == 0 {
            Self::ToMemory
        } else {
            Self::FromMemory
        }
    }

    fn from_op_bit5(op: u8) -> Self {
        if op & 0b0001_0000 == 0 {
            Self::ToMemory
        } else {
            Self::FromMemory
        }
    }
}

#[derive(Debug)]
pub enum LoadMemoryLocation {
    Bc,
    De,
    HlInc,
    HlDec,
    ZeroPageC,
    ZeroPageImm(u8),
    Absolute(u16),
}

impl LoadMemoryLocation {
    fn from_op_bits56(op: u8) -> Self {
        match op & 0b0011_0000 {
            0x00 => Self::Bc,
            0x10 => Self::De,
            0x20 => Self::HlInc,
            0x30 => Self::HlDec,
            masked => panic!("Impossible LoadMemoryLocation {masked} op {op:02x}"),
        }
    }

    fn from_immediate8(reader: &mut PcReader) -> Self {
        Self::ZeroPageImm(reader.read_u8())
    }

    fn from_immediate16(reader: &mut PcReader) -> Self {
        Self::Absolute(reader.read_u16())
    }
}

#[derive(Debug)]
pub enum Condition {
    Always,
    NonZero,
    NonCarry,
    Zero,
    Carry,
}

impl Condition {
    fn from_op_bits156(op: u8) -> Self {
        match op & 0b0001_1001 {
            0x00 => Self::NonZero,
            0x10 => Self::NonCarry,
            0x08 => Self::Zero,
            0x18 => Self::Carry,
            0x01 | 0x09 => Self::Always,
            masked => panic!("Impossible Condition 0x{masked:02} in op {op:02x}"),
        }
    }
}

#[derive(Debug)]
pub enum Destination {
    Hl,
    Relative(i8),
    Absolute(u16),
}

impl Destination {
    fn from_immediate8(reader: &mut PcReader) -> Self {
        let immediate = reader.read_u8();
        Self::Relative(immediate as i8)
    }

    fn from_immediate16(reader: &mut PcReader) -> Self {
        Self::Absolute(reader.read_u16())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ResetSlot {
    Slot0,
    Slot1,
    Slot2,
    Slot3,
    Slot4,
    Slot5,
    Slot6,
    Slot7,
}

impl ResetSlot {
    fn from_op(op: u8) -> Self {
        match op {
            0xc7 => Self::Slot0,
            0xcf => Self::Slot1,
            0xd7 => Self::Slot2,
            0xdf => Self::Slot3,
            0xe7 => Self::Slot4,
            0xef => Self::Slot5,
            0xf7 => Self::Slot6,
            0xff => Self::Slot7,
            _ => panic!("Impossible Reset Operation {op:02x}"),
        }
    }
}

#[derive(Debug)]
pub enum AddToSpDestination {
    Sp,
    Hl,
}

impl AddToSpDestination {
    fn from_op(op: u8) -> Self {
        match op {
            0xe8 => Self::Sp,
            0xf8 => Self::Hl,
            _ => panic!("Impossible AddToSpDestination {op:02x}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Bit {
    Bit0,
    Bit1,
    Bit2,
    Bit3,
    Bit4,
    Bit5,
    Bit6,
    Bit7,
}

impl Bit {
    fn from_op_bits456(op: u8) -> Self {
        match op & 0b0011_1000 {
            0x00 => Self::Bit0,
            0x08 => Self::Bit1,
            0x10 => Self::Bit2,
            0x18 => Self::Bit3,
            0x20 => Self::Bit4,
            0x28 => Self::Bit5,
            0x30 => Self::Bit6,
            0x38 => Self::Bit7,
            _ => panic!("Impossible Bit {op:02x}"),
        }
    }

    pub fn mask(&self) -> u8 {
        1 << (*self as u8)
    }
}

#[derive(Debug)]
pub enum BitOp {
    Rotate(RotateDirection, RotateCarry),
    ShiftArithmetic(RotateDirection),
    SwapNibbles,
    ShiftRightLogical,
    Test(Bit),
    Clear(Bit),
    Set(Bit),
}

impl BitOp {
    fn from_sub_op(reader: &mut PcReader) -> (Self, Operand8) {
        let sub_op = reader.read_u8();

        let operand = Operand8::from_op_bits123(sub_op);
        let bit = Bit::from_op_bits456(sub_op);

        let operation = match sub_op {
            0x00..=0x07 => Self::Rotate(RotateDirection::Left, RotateCarry::Through),
            0x08..=0x0f => Self::Rotate(RotateDirection::Right, RotateCarry::Through),
            0x10..=0x17 => Self::Rotate(RotateDirection::Left, RotateCarry::NotThrough),
            0x18..=0x1f => Self::Rotate(RotateDirection::Right, RotateCarry::NotThrough),
            0x20..=0x27 => Self::ShiftArithmetic(RotateDirection::Left),
            0x28..=0x2f => Self::ShiftArithmetic(RotateDirection::Right),
            0x30..=0x37 => Self::SwapNibbles,
            0x38..=0x3f => Self::ShiftRightLogical,
            0x40..=0x7f => Self::Test(bit),
            0x80..=0xbf => Self::Clear(bit),
            0xc0..=0xff => Self::Set(bit),
        };

        (operation, operand)
    }
}

#[derive(Debug)]
pub enum Instruction {
    Add16(Operand16),
    AddToSp(AddToSpDestination, i8),
    ArithmeticLogic8(ArithmeticLogic, Operand8),
    BitOp(BitOp, Operand8),
    Call(Destination, Condition),
    Ccf,
    Cpl,
    Daa,
    Di,
    Ei,
    IncDec16(IncDecDirection, Operand16),
    IncDec8(IncDecDirection, Operand8),
    Invalid(u8),
    Halt,
    Jump(Destination, Condition),
    LoadHlToSp,
    LoadImm16(Operand16, u16),
    LoadMemory(LoadMemoryDirection, LoadMemoryLocation),
    LoadSimple(Operand8, Operand8),
    LoadSpToImm(u16),
    Nop,
    Pop(Operand16),
    Push(Operand16),
    Reset(ResetSlot),
    Reti,
    Return(Condition),
    RotateA(RotateDirection, RotateCarry),
    Scf,
    Stop,
}

impl Instruction {
    fn from_simple_op(op: u8) -> Self {
        match op {
            0x00 => Self::Nop,
            0x10 => Self::Stop,
            0x27 => Self::Daa,
            0x2f => Self::Cpl,
            0x37 => Self::Scf,
            0x3f => Self::Ccf,
            0x76 => Self::Halt,
            0xd9 => Self::Reti,
            0xf3 => Self::Di,
            0xf9 => Self::LoadHlToSp,
            0xfb => Self::Ei,
            _ => panic!("Invalid simple instruction {op:02x}"),
        }
    }

    pub fn from_pc_reader(reader: &mut PcReader) -> Self {
        let op = reader.read_u8();

        match op {
            0x00 | 0x10 | 0x27 | 0x2f | 0x37 | 0x3f | 0x76 | 0xd9 | 0xf3 | 0xf9 | 0xfb => {
                Self::from_simple_op(op)
            }
            0x01 | 0x11 | 0x21 | 0x31 => {
                let operand = Operand16::from_op_bits56_sp(op);
                let immediate = reader.read_u16();
                Self::LoadImm16(operand, immediate)
            }
            0x02 | 0x12 | 0x22 | 0x32 | 0x0a | 0x1a | 0x2a | 0x3a => {
                let direction = LoadMemoryDirection::from_op_bit4(op);
                let location = LoadMemoryLocation::from_op_bits56(op);
                Self::LoadMemory(direction, location)
            }
            0x03 | 0x13 | 0x23 | 0x33 | 0x0b | 0x1b | 0x2b | 0x3b => {
                let direction = IncDecDirection::from_op_bit4(op);
                let operand = Operand16::from_op_bits56_sp(op);
                Self::IncDec16(direction, operand)
            }
            0x04 | 0x05 | 0x14 | 0x15 | 0x24 | 0x25 | 0x34 | 0x35 | 0x0c | 0x0d | 0x1c | 0x1d
            | 0x2c | 0x2d | 0x3c | 0x3d => {
                let direction = IncDecDirection::from_op_bit1(op);
                let operand = Operand8::from_op_bits456(op);
                Self::IncDec8(direction, operand)
            }
            0x06 | 0x16 | 0x26 | 0x36 | 0x0e | 0x1e | 0x2e | 0x3e => {
                let source = Operand8::from_immediate(reader);
                let destination = Operand8::from_op_bits456(op);
                Self::LoadSimple(destination, source)
            }
            0x07 | 0x0f | 0x17 | 0x1f => {
                let direction = RotateDirection::from_op_bit4(op);
                let carry = RotateCarry::from_op_bit5(op);
                Self::RotateA(direction, carry)
            }
            0x08 => {
                let immediate = reader.read_u16();
                Self::LoadSpToImm(immediate)
            }
            0x09 | 0x19 | 0x29 | 0x39 => {
                let operand = Operand16::from_op_bits56_sp(op);
                Self::Add16(operand)
            }
            0x18 | 0x20 | 0x30 | 0x28 | 0x38 => {
                let destination = Destination::from_immediate8(reader);
                let condition = match op {
                    0x18 => Condition::Always,
                    _ => Condition::from_op_bits156(op),
                };
                Self::Jump(destination, condition)
            }
            0x40..=0x7f => {
                let source = Operand8::from_op_bits123(op);
                let destination = Operand8::from_op_bits456(op);
                Self::LoadSimple(destination, source)
            }
            0x80..=0xbf => {
                let operation = ArithmeticLogic::from_op_bits456(op);
                let operand = Operand8::from_op_bits123(op);
                Self::ArithmeticLogic8(operation, operand)
            }
            0xc0 | 0xd0 | 0xc8 | 0xd8 | 0xc9 => {
                let condition = Condition::from_op_bits156(op);
                Self::Return(condition)
            }
            0xc1 | 0xd1 | 0xe1 | 0xf1 => {
                let operand = Operand16::from_op_bits56_af(op);
                Self::Pop(operand)
            }
            0xc2 | 0xd2 | 0xca | 0xda | 0xc3 => {
                let destination = Destination::from_immediate16(reader);
                let condition = Condition::from_op_bits156(op);
                Self::Jump(destination, condition)
            }
            0xc4 | 0xd4 | 0xcc | 0xdc | 0xcd => {
                let destination = Destination::from_immediate16(reader);
                let condition = Condition::from_op_bits156(op);
                Self::Call(destination, condition)
            }
            0xc5 | 0xd5 | 0xe5 | 0xf5 => {
                let operand = Operand16::from_op_bits56_af(op);
                Self::Push(operand)
            }
            0xc6 | 0xd6 | 0xe6 | 0xf6 | 0xce | 0xde | 0xee | 0xfe => {
                let operation = ArithmeticLogic::from_op_bits456(op);
                let operand = Operand8::from_immediate(reader);
                Self::ArithmeticLogic8(operation, operand)
            }
            0xc7 | 0xcf | 0xd7 | 0xdf | 0xe7 | 0xef | 0xf7 | 0xff => {
                let slot = ResetSlot::from_op(op);
                Self::Reset(slot)
            }
            0xcb => {
                let (bit_op, operand) = BitOp::from_sub_op(reader);
                Self::BitOp(bit_op, operand)
            }
            0xe0 | 0xf0 => {
                let direction = LoadMemoryDirection::from_op_bit5(op);
                let location = LoadMemoryLocation::from_immediate8(reader);
                Self::LoadMemory(direction, location)
            }
            0xe2 | 0xf2 => {
                let direction = LoadMemoryDirection::from_op_bit5(op);
                let location = LoadMemoryLocation::ZeroPageC;
                Self::LoadMemory(direction, location)
            }
            0xe8 | 0xf8 => {
                let immediate = reader.read_u8();
                let destination = AddToSpDestination::from_op(op);
                Self::AddToSp(destination, immediate as i8)
            }
            0xe9 => {
                let destination = Destination::Hl;
                let condition = Condition::Always;
                Self::Jump(destination, condition)
            }
            0xea | 0xfa => {
                let direction = LoadMemoryDirection::from_op_bit5(op);
                let location = LoadMemoryLocation::from_immediate16(reader);
                Self::LoadMemory(direction, location)
            }
            0xd3 | 0xdb | 0xdd | 0xe3 | 0xe4 | 0xeb | 0xec | 0xed | 0xf4 | 0xfc | 0xfd => {
                Self::Invalid(op)
            }
        }
    }
}
