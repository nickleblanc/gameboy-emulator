use std;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlagsRegister {
    pub z: bool,
    pub n: bool,
    pub h: bool,
    pub c: bool,
}

const ZERO_FLAG_BYTE_POSITION: u8 = 7;
const SUBTRACT_FLAG_BYTE_POSITION: u8 = 6;
const HALF_CARRY_FLAG_BYTE_POSITION: u8 = 5;
const CARRY_FLAG_BYTE_POSITION: u8 = 4;

impl FlagsRegister {
    pub fn new() -> FlagsRegister {
        FlagsRegister {
            z: true,
            n: false,
            h: false,
            c: false,
        }
    }
}

impl std::convert::From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        let z = ((byte >> ZERO_FLAG_BYTE_POSITION) & 0b1) != 0;
        let n = ((byte >> SUBTRACT_FLAG_BYTE_POSITION) & 0b1) != 0;
        let h = ((byte >> HALF_CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;
        let c = ((byte >> CARRY_FLAG_BYTE_POSITION) & 0b1) != 0;

        FlagsRegister { z, n, h, c }
    }
}

impl std::convert::From<FlagsRegister> for u8 {
    fn from(flag: FlagsRegister) -> u8 {
        (if flag.z { 1 } else { 0 }) << ZERO_FLAG_BYTE_POSITION
            | (if flag.n { 1 } else { 0 }) << SUBTRACT_FLAG_BYTE_POSITION
            | (if flag.h { 1 } else { 0 }) << HALF_CARRY_FLAG_BYTE_POSITION
            | (if flag.c { 1 } else { 0 }) << CARRY_FLAG_BYTE_POSITION
    }
}
