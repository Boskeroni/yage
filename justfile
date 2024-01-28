set shell := ["cmd.exe", "/c"]

test TEST:
    cargo run blarggs/{{TEST}}
boot:
    cargo run blarggs/dmg_rom.bin unbooted
test_all:
    cargo run blarggs/cpu_instrs.gb

doctor TEST:
    python ./gameboy-doctor/gameboy-doctor gameboy_doctor.txt cpu_instrs {{TEST}}

play ROM:
    cargo run roms/{{ROM}}.gb
