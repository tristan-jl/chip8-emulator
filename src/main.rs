use chip8::{run_chip8, Chip8};
use clap::Parser;

/// Chip8 emulator
#[derive(Parser, Debug)]
#[command(author, version,about, long_about=None)]
struct Args {
    /// Rom path
    #[arg(short, long)]
    rom_path: String,

    /// Cycle delay in milliseconds
    #[arg(short, long, default_value_t = 10)]
    cycle_delay: u32,
}

fn main() {
    env_logger::init();
    let sdl_context = sdl2::init().unwrap();

    let args = Args::parse();

    let chip8 = Chip8::read_rom(&args.rom_path).unwrap();
    run_chip8(sdl_context, chip8, args.cycle_delay);
}
