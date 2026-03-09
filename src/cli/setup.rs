use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use dialoguer::{Confirm, Select};

#[derive(Debug, Clone, Copy)]
pub enum Agent {
    ClaudeCode,
    OpenCode,
    Cursor,
    Windsurf,
    VsCode,
    GeminiCli,
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Agent::ClaudeCode => write!(f, "Claude Code"),
            Agent::OpenCode => write!(f, "OpenCode"),
            Agent::Cursor => write!(f, "Cursor"),
            Agent::Windsurf => write!(f, "Windsurf"),
            Agent::VsCode => write!(f, "VS Code"),
            Agent::GeminiCli => write!(f, "Gemini CLI"),
        }
    }
}

const ALL_AGENTS: &[Agent] = &[
    Agent::ClaudeCode,
    Agent::OpenCode,
    Agent::Cursor,
    Agent::Windsurf,
    Agent::VsCode,
    Agent::GeminiCli,
];

impl Agent {
    fn config_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        Some(match self {
            Agent::ClaudeCode => home.join(".claude").join("settings.json"),
            Agent::OpenCode => home.join(".config").join("opencode").join("config.json"),
            Agent::Cursor => home.join(".cursor").join("mcp.json"),
            Agent::Windsurf => home.join(".codeium").join("windsurf").join("mcp_config.json"),
            Agent::VsCode => std::env::current_dir()
                .unwrap_or_default()
                .join(".vscode")
                .join("mcp.json"),
            Agent::GeminiCli => home.join(".gemini").join("settings.json"),
        })
    }

    fn supports_hooks(&self) -> bool {
        matches!(self, Agent::ClaudeCode)
    }
}

fn mcp_config_value() -> serde_json::Value {
    serde_json::json!({
        "command": "cortexmem",
        "args": ["mcp"],
        "type": "stdio"
    })
}

fn write_mcp_config(agent: Agent) -> Result<()> {
    let path = agent.config_path().context("Could not determine home directory")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut config: serde_json::Value = if path.exists() {
        let contents = std::fs::read_to_string(&path)?;
        serde_json::from_str(&contents).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let servers = config
        .as_object_mut()
        .context("Config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    if servers.get("cortexmem").is_some() {
        let overwrite = Confirm::new()
            .with_prompt("cortexmem is already configured. Overwrite?")
            .default(false)
            .interact()?;
        if !overwrite {
            println!("Skipped MCP config (already exists).");
            return Ok(());
        }
    }

    servers
        .as_object_mut()
        .context("mcpServers is not a JSON object")?
        .insert("cortexmem".into(), mcp_config_value());

    let formatted = serde_json::to_string_pretty(&config)?;
    std::fs::write(&path, formatted)?;
    println!("MCP config written to {}", path.display());
    Ok(())
}

fn install_claude_plugin() -> Result<()> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let claude_dir = home.join(".claude");

    let hooks_json = include_str!("../../plugin/hooks/hooks.json");
    let session_start = include_str!("../../plugin/scripts/session-start.sh");
    let session_end = include_str!("../../plugin/scripts/session-end.sh");
    let compaction_recovery = include_str!("../../plugin/scripts/compaction-recovery.sh");
    let skill_md = include_str!("../../plugin/skills/memory-protocol/SKILL.md");

    let hooks_dir = claude_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;
    std::fs::write(hooks_dir.join("cortexmem.json"), hooks_json)?;
    println!("  Hooks installed to {}", hooks_dir.display());

    let scripts_dir = claude_dir.join("scripts").join("cortexmem");
    std::fs::create_dir_all(&scripts_dir)?;
    std::fs::write(scripts_dir.join("session-start.sh"), session_start)?;
    std::fs::write(scripts_dir.join("session-end.sh"), session_end)?;
    std::fs::write(scripts_dir.join("compaction-recovery.sh"), compaction_recovery)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for script in &["session-start.sh", "session-end.sh", "compaction-recovery.sh"] {
            let path = scripts_dir.join(script);
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms)?;
        }
    }
    println!("  Scripts installed to {}", scripts_dir.display());

    let skill_dir = claude_dir.join("skills").join("cortexmem");
    std::fs::create_dir_all(&skill_dir)?;
    std::fs::write(skill_dir.join("SKILL.md"), skill_md)?;
    println!("  Memory Protocol skill installed to {}", skill_dir.display());

    Ok(())
}

pub fn run_setup() -> Result<()> {
    println!("cortexmem setup\n");

    let items: Vec<String> = ALL_AGENTS.iter().map(|a| a.to_string()).collect();
    let selection = Select::new()
        .with_prompt("Which AI agent do you use?")
        .items(&items)
        .default(0)
        .interact()?;

    let agent = ALL_AGENTS[selection];
    println!("\nConfiguring cortexmem for {}...\n", agent);

    write_mcp_config(agent)?;

    if agent.supports_hooks() {
        println!("\nInstalling Claude Code plugin files...");
        install_claude_plugin()?;
    }

    println!("\nSetup complete! Restart {} to activate cortexmem.", agent);
    Ok(())
}
