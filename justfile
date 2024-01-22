set shell := ["cmd.exe", "/c"]

test TEST:
    cargo run blarggs/{{TEST}}
doctor TEST:
    python ./gameboy-doctor/gameboy-doctor gameboy_doctor.txt cpu_instrs {{TEST}}
rom:
    cargo run rom/dmb_rom.bin
play ROM:
    cargo run roms/{{ROM}}.gb
release:
    cargo build --release
