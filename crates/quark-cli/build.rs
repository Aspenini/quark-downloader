//! Generates shell completions and the man page into `$OUT_DIR/assets/` at
//! build time (no runtime cost). Packagers can locate the directory with:
//! `find target -type d -path '*/quark-cli-*/out/assets'`.

use clap::CommandFactory;
use clap_complete::shells::{Bash, Elvish, Fish, PowerShell, Zsh};

include!("src/cli.rs");

fn main() {
    println!("cargo:rerun-if-changed=src/cli.rs");
    let out =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("assets");
    std::fs::create_dir_all(&out).expect("create assets dir");

    let mut cmd = Cli::command();
    let name = "quark-downloader";
    clap_complete::generate_to(Bash, &mut cmd, name, &out).expect("bash completions");
    clap_complete::generate_to(Zsh, &mut cmd, name, &out).expect("zsh completions");
    clap_complete::generate_to(Fish, &mut cmd, name, &out).expect("fish completions");
    clap_complete::generate_to(PowerShell, &mut cmd, name, &out).expect("powershell completions");
    clap_complete::generate_to(Elvish, &mut cmd, name, &out).expect("elvish completions");

    let mut man = Vec::new();
    clap_mangen::Man::new(cmd)
        .render(&mut man)
        .expect("man page");
    std::fs::write(out.join("quark-downloader.1"), man).expect("write man page");
}
