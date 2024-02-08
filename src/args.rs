use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, allow_hyphen_values(true))]
pub struct Args {
    pub rom_name: String,

    #[arg(short, long)]
    pub booted: bool,
    #[arg(short, long)]
    pub save: bool,  
}
