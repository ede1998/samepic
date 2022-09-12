use clap::{crate_name, Args, CommandFactory};
use clap_complete::{generate, Shell};

/// Prints completions for the given shell
#[derive(Debug, Args)]
pub struct Completions {
    /// Shell to print completions for
    #[clap(value_enum)]
    shell: Shell,
}

impl Completions {
    pub fn run(self) {
        generate(
            self.shell,
            &mut crate::Args::command(),
            crate_name!(),
            &mut std::io::stdout(),
        )
    }
}
