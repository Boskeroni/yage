use crate::memory::Memory;

enum TimerRegisters {
    DIV=0xFF04,
    TIMA=0xFF05,
    TMA=0xFF06,
    TAC=0xFF07,
}

pub fn update_timer(memory: &mut Memory, cycles: u8) {
    use TimerRegisters::*;

    let tac = memory.read(TAC as u16);

    let timer_enable = tac & 0b0000_0100;
    let bit_position = match tac & 0b0000_0011 {
        0 => 9,
        1 => 3,
        2 => 5,
        3 => 7,
        _ => unreachable!(),
    };

    let mut whole_div = (memory.read(DIV as u16) as u16) << 8 | memory.div as u16;
    let mut prev_edge = true;

    for _ in 0..cycles {
        // div is incremented
        whole_div = whole_div.wrapping_add(1);
        memory.unchecked_write(DIV as u16, (whole_div>>8) as u8);

        let anded_result = ((whole_div & 1<<bit_position)!=0)&&(timer_enable!=0);
        if prev_edge && !anded_result {
            // for the next cycle
            prev_edge = anded_result;
            
            let tima = memory.read(TIMA as u16);
            let (new_tima, overflow) = tima.overflowing_add(1);

            if overflow {
                // the value it resets to when overlfowing
                let tma = memory.read(TMA as u16);
                memory.write(TIMA as u16, tma);

                //call the interrupt
                let i_flag = memory.read(0xFF0F);
                memory.write(0xFF0F, i_flag|0b0000_0100);
                continue;
            }
            // just a normal increment
            memory.write(TIMA as u16, new_tima);
        }
    }
}