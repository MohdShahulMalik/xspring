use clap::{ArgAction, ArgGroup, Subcommand};

#[derive(Subcommand)]
pub enum Commands{
    Quick{
        #[arg(short = 'm', long, action = ArgAction::SetTrue)]
        maven: bool,

        #[arg(short = 'e', long, action = ArgAction::SetTrue)]
        extended: bool,
    },

    #[command(group(
        ArgGroup::new("list_item")
            .required(true)
            .args(["java", "boot", "project_type", "language", "deps"]),
    ))]
    List {
        #[arg(short = 'j', long, action = ArgAction::SetTrue)]
        java: bool,

        #[arg(short = 'b', long, action = ArgAction::SetTrue)]
        boot: bool,

        #[arg(short = 't', long = "type", action = ArgAction::SetTrue)]
        project_type: bool,

        #[arg(short = 'l', long, action = ArgAction::SetTrue)]
        language: bool,

        #[arg(short = 'd', long, action = ArgAction::SetTrue)]
        deps: bool,
    }
}
