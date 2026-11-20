// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

use serde::{Deserialize, Serialize};

/// Shell completion generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionGenerator {
    pub shell: CompletionShell,
    pub binary_name: String,
}

/// Target shell for completions
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

impl std::fmt::Display for CompletionShell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionShell::Bash => write!(f, "bash"),
            CompletionShell::Zsh => write!(f, "zsh"),
            CompletionShell::Fish => write!(f, "fish"),
            CompletionShell::PowerShell => write!(f, "powershell"),
            CompletionShell::Elvish => write!(f, "elvish"),
        }
    }
}

impl CompletionShell {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "bash" => Some(CompletionShell::Bash),
            "zsh" => Some(CompletionShell::Zsh),
            "fish" => Some(CompletionShell::Fish),
            "powershell" | "pwsh" => Some(CompletionShell::PowerShell),
            "elvish" => Some(CompletionShell::Elvish),
            _ => None,
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            CompletionShell::Bash => "bash",
            CompletionShell::Zsh => "zsh",
            CompletionShell::Fish => "fish",
            CompletionShell::PowerShell => "ps1",
            CompletionShell::Elvish => "elv",
        }
    }

    pub fn install_instructions(&self) -> String {
        match self {
            CompletionShell::Bash => "# Add to ~/.bashrc:\n\
                 source <(veyron completions bash)\n\
                 # Or save to file:\n\
                 veyron completions bash > /etc/bash_completion.d/veyron"
                .to_string(),
            CompletionShell::Zsh => "# Add to ~/.zshrc:\n\
                 source <(veyron completions zsh)\n\
                 # Or save to file:\n\
                 veyron completions zsh > ~/.zsh/completions/_veyron"
                .to_string(),
            CompletionShell::Fish => "# Save to fish completions directory:\n\
                 veyron completions fish > ~/.config/fish/completions/veyron.fish"
                .to_string(),
            CompletionShell::PowerShell => "# Add to PowerShell profile:\n\
                 veyron completions powershell | Out-String | Invoke-Expression"
                .to_string(),
            CompletionShell::Elvish => "# Add to ~/.elvish/rc.elv:\n\
                 eval (veyron completions elvish | slurp)"
                .to_string(),
        }
    }
}

impl CompletionGenerator {
    pub fn new(shell: CompletionShell) -> Self {
        Self {
            shell,
            binary_name: "veyron".to_string(),
        }
    }

    pub fn with_binary_name(mut self, name: impl Into<String>) -> Self {
        self.binary_name = name.into();
        self
    }

    pub fn generate(&self) -> String {
        use clap::CommandFactory;
        use clap_complete::{Shell, generate as gen_completion};

        let shell = match self.shell {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
            CompletionShell::PowerShell => Shell::PowerShell,
            CompletionShell::Elvish => Shell::Elvish,
        };

        let mut cmd = crate::cli::Cli::command();
        let mut buf = Vec::new();
        gen_completion(shell, &mut cmd, &self.binary_name, &mut buf);
        String::from_utf8(buf).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_shell_from_str() {
        assert_eq!(CompletionShell::parse("bash"), Some(CompletionShell::Bash));
        assert_eq!(CompletionShell::parse("zsh"), Some(CompletionShell::Zsh));
        assert_eq!(CompletionShell::parse("fish"), Some(CompletionShell::Fish));
        assert_eq!(
            CompletionShell::parse("powershell"),
            Some(CompletionShell::PowerShell)
        );
        assert_eq!(
            CompletionShell::parse("pwsh"),
            Some(CompletionShell::PowerShell)
        );
        assert_eq!(
            CompletionShell::parse("elvish"),
            Some(CompletionShell::Elvish)
        );
        assert_eq!(CompletionShell::parse("unknown"), None);
    }

    #[test]
    fn test_completion_shell_display() {
        assert_eq!(CompletionShell::Bash.to_string(), "bash");
        assert_eq!(CompletionShell::Zsh.to_string(), "zsh");
        assert_eq!(CompletionShell::Fish.to_string(), "fish");
        assert_eq!(CompletionShell::PowerShell.to_string(), "powershell");
        assert_eq!(CompletionShell::Elvish.to_string(), "elvish");
    }

    #[test]
    fn test_completion_shell_file_extension() {
        assert_eq!(CompletionShell::Bash.file_extension(), "bash");
        assert_eq!(CompletionShell::Zsh.file_extension(), "zsh");
        assert_eq!(CompletionShell::Fish.file_extension(), "fish");
        assert_eq!(CompletionShell::PowerShell.file_extension(), "ps1");
        assert_eq!(CompletionShell::Elvish.file_extension(), "elv");
    }

    #[test]
    fn test_completion_generator_new() {
        let cgen = CompletionGenerator::new(CompletionShell::Bash);
        assert_eq!(cgen.shell, CompletionShell::Bash);
        assert_eq!(cgen.binary_name, "veyron");
    }

    #[test]
    fn test_completion_generator_with_binary_name() {
        let cgen = CompletionGenerator::new(CompletionShell::Zsh).with_binary_name("vc");
        assert_eq!(cgen.binary_name, "vc");
    }

    /// Helper to run completion generation on a thread with a larger stack,
    /// since the large Commands enum (146 variants) causes deep recursion
    /// in clap's command builder that exceeds the default test thread stack.
    fn run_with_large_stack<F: FnOnce() + Send + 'static>(f: F) {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024) // 16 MiB
            .spawn(f)
            .expect("failed to spawn thread")
            .join()
            .expect("thread panicked");
    }

    #[test]
    fn test_generate_bash_completions() {
        run_with_large_stack(|| {
            let cgen = CompletionGenerator::new(CompletionShell::Bash);
            let output = cgen.generate();
            assert!(!output.is_empty(), "Bash completions should not be empty");
            assert!(output.contains("veyron"));
        });
    }

    #[test]
    fn test_generate_zsh_completions() {
        run_with_large_stack(|| {
            let cgen = CompletionGenerator::new(CompletionShell::Zsh);
            let output = cgen.generate();
            assert!(!output.is_empty(), "Zsh completions should not be empty");
            assert!(output.contains("veyron"));
        });
    }

    #[test]
    fn test_generate_fish_completions() {
        run_with_large_stack(|| {
            let cgen = CompletionGenerator::new(CompletionShell::Fish);
            let output = cgen.generate();
            assert!(!output.is_empty(), "Fish completions should not be empty");
            assert!(output.contains("veyron"));
        });
    }

    #[test]
    fn test_generate_powershell_completions() {
        run_with_large_stack(|| {
            let cgen = CompletionGenerator::new(CompletionShell::PowerShell);
            let output = cgen.generate();
            assert!(
                !output.is_empty(),
                "PowerShell completions should not be empty"
            );
            assert!(output.contains("veyron"));
        });
    }

    #[test]
    fn test_generate_elvish_completions() {
        run_with_large_stack(|| {
            let cgen = CompletionGenerator::new(CompletionShell::Elvish);
            let output = cgen.generate();
            assert!(!output.is_empty(), "Elvish completions should not be empty");
            assert!(output.contains("veyron"));
        });
    }

    #[test]
    fn test_install_instructions() {
        let bash = CompletionShell::Bash.install_instructions();
        assert!(bash.contains("bashrc"));

        let zsh = CompletionShell::Zsh.install_instructions();
        assert!(zsh.contains("zshrc"));

        let fish = CompletionShell::Fish.install_instructions();
        assert!(fish.contains("fish"));

        let ps = CompletionShell::PowerShell.install_instructions();
        assert!(ps.contains("PowerShell"));

        let elv = CompletionShell::Elvish.install_instructions();
        assert!(elv.contains("elvish"));
    }
}
