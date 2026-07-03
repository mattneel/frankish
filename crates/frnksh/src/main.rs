//! frnksh — the frankish driver (SPEC §9). Bare invocation becomes the REPL
//! ("the frankish shell") at M8; subcommands accrete with their milestones.
//! The `frankish` alias symlink (D-002) ships with packaging, not before.
//!
//! Until M8 this binary only identifies the build, so that the workspace
//! skeleton carries no dead CLI scaffolding ahead of its milestone.

fn version_line() -> String {
    format!(
        "frnksh {} (pre-M8 skeleton; the shell lands at M8 — docs/SPEC.md §9)",
        env!("CARGO_PKG_VERSION")
    )
}

fn main() {
    println!("{}", version_line());
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_line_names_the_binary_and_version() {
        let line = super::version_line();
        assert!(line.starts_with("frnksh "));
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
    }
}
