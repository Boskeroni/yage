use crate::memory::Memory;

const Comparison_frequency: u32 = 4194304;

enum TimerRegisters {
    DIV=0xFF04,
    TIMA=0xFF05,
    TMA=0xFF06,
    TAC=0xFF07,
}

pub fn update_timer(timer: &mut u32, memory: &mut Memory, cycles: u8) {
    use TimerRegisters::*;

    // updating the div register
    let (new, overflow) = memory.div.overflowing_add(cycles/4);
    memory.div = new;
    if overflow {
        let upper_div = memory.read(DIV as u16);
        memory.unchecked_write(DIV as u16, upper_div.wrapping_add(1));
    }

    let tac = memory.read(TAC as u16);

    // the tima doesnt need to be updated
    if tac & 0b0000_0100 == 0 {
        return;
    }

    let tac = tac & 0b0000_0011;

    let bit_position = match tac {
        0 => 9,
        1 => 3,
        2 => 5,
        3 => 7,
        _ => panic!("genuinely impossible"),
    };

    let hz = 1<<bit_position;
}