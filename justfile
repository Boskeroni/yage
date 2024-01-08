set shell := ["cmd.exe", "/c"]

test TEST:
    cargo run blarggs/{{TEST}}

doctor TEST:
    python ./gameboy-doctor/gameboy-doctor gameboy_doctor.txt cpu_instrs {{TEST}}



release:
    cargo build --release

run ROM:
    cargo run roms/{{ROM}}.gb