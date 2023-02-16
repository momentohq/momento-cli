use momento_cli_opts::Momento;

use std::env;
use std::error::Error;
use std::path::Path;

use clap_complete::generate_to;
use clap_complete::shells::{Bash, Zsh};

fn main() -> Result<(), Box<dyn Error>> {
    let mut command = Momento::meta_command();

    // OUT_DIR is a unique output dir for the crate, with a pseudo-random name.
    let out_dir = env::var("OUT_DIR")?;
    // we want to write the completion file to the "target dir", where the binary
    // is written, so that it's easier to find it when we are creating installers
    // and such.  cargo does not currently provide us an env var for target dir,
    // so we compute it.  (It happens to be 3 dirs up from OUT_DIR).
    let target_dir = Path::new(&out_dir).join("../../..");
    generate_to(Zsh, &mut command, "momento", &target_dir)?;
    generate_to(Bash, &mut command, "momento", &target_dir)?;

    Ok(())
}
