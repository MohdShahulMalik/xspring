use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands{
    Quick{
        #[arg(short = 'n', long)]
        name: String,
    }
}
