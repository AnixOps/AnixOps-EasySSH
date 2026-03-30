use easyssh_core::db::{Database, NewGroup, NewServer};
use easyssh_core::terminal;
use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        print_help();
        return Ok(());
    }

    run_cli(&args)
}

fn run_cli(args: &[String]) -> Result<(), Box<dyn Error>> {
    let db = Database::new(easyssh_core::get_db_path())?;
    db.init()?;

    match args[1].as_str() {
        "add-server" | "add" => add_server(&db, args),
        "add-group" | "group" => add_group(&db, args),
        "list" | "ls" => list_servers(&db),
        "import-ssh" | "import" => import_ssh_config(&db),
        "connect" => connect_server(&db, args),
        "debug-server" => start_debug_server(args),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("Unknown command: {other}");
            print_help();
            Ok(())
        }
    }
}

fn add_server(db: &Database, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 5 {
        eprintln!("Usage: easyssh add-server <name> <host> <username> [port] [auth_type]");
        return Ok(());
    }

    let name = &args[2];
    let host = &args[3];
    let username = &args[4];
    let port_str = args.get(5).cloned().unwrap_or_else(|| "22".to_string());
    let auth_type = args.get(6).cloned().unwrap_or_else(|| "agent".to_string());

    validate_server(name, host, username, &port_str, &auth_type)?;
    let port = port_str.parse::<i64>()?;

    let server = NewServer {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
        host: host.clone(),
        port,
        username: username.clone(),
        auth_type,
        identity_file: None,
        group_id: None,
        status: "unknown".to_string(),
    };

    db.add_server(&server)?;
    println!("Added server: {name}");
    Ok(())
}

fn add_group(db: &Database, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 {
        eprintln!("Usage: easyssh add-group <name>");
        return Ok(());
    }

    let name = &args[2];
    validate_group_name(name)?;

    let group = NewGroup {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.clone(),
    };

    db.add_group(&group)?;
    println!("Added group: {name}");
    Ok(())
}

fn list_servers(db: &Database) -> Result<(), Box<dyn Error>> {
    let servers = db.get_servers()?;
    let groups = db.get_groups()?;

    if servers.is_empty() {
        println!("No servers configured");
        return Ok(());
    }

    for s in servers {
        let group_name = s
            .group_id
            .as_ref()
            .and_then(|gid| groups.iter().find(|g| &g.id == gid).map(|g| g.name.as_str()))
            .unwrap_or("(ungrouped)");

        println!("[{group_name}] {} ({}@{}:{})", s.name, s.username, s.host, s.port);
    }

    Ok(())
}

fn import_ssh_config(db: &Database) -> Result<(), Box<dyn Error>> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let config_path: PathBuf = home.join(".ssh").join("config");

    if !config_path.exists() {
        eprintln!("~/.ssh/config not found");
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)?;
    let mut current_host: Option<String> = None;
    let mut imported = 0;

    for line in content.lines() {
        let line = line.trim();
        if let Some(host) = line.strip_prefix("Host ") {
            if !host.contains('*') {
                current_host = Some(host.trim().to_string());
            }
        } else if let Some(ref host) = current_host {
            if let Some(hostname) = line.strip_prefix("HostName ") {
                let server = NewServer {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: host.clone(),
                    host: hostname.trim().to_string(),
                    port: 22,
                    username: whoami::username(),
                    auth_type: "agent".to_string(),
                    identity_file: None,
                    group_id: None,
                    status: "unknown".to_string(),
                };
                if db.add_server(&server).is_ok() {
                    imported += 1;
                }
                current_host = None;
            }
        }
    }

    println!("Imported {imported} servers from ~/.ssh/config");
    Ok(())
}

fn connect_server(db: &Database, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() < 3 {
        eprintln!("Usage: easyssh connect <server-id>");
        return Ok(());
    }

    let server = db.get_server(&args[2])?;
    terminal::open_native_terminal(
        &server.host,
        server.port as u16,
        &server.username,
        &server.auth_type,
    )?;
    Ok(())
}

fn start_debug_server(args: &[String]) -> Result<(), Box<dyn Error>> {
    if matches!(args.get(2).map(String::as_str), Some("-h" | "--help")) {
        println!("Usage: easyssh debug-server [host] [port]");
        println!("Defaults to 127.0.0.1:7878 and stays loopback-only.");
        return Ok(());
    }

    let host = args.get(2).cloned().unwrap_or_else(|| "127.0.0.1".to_string());
    let port = args.get(3).and_then(|p| p.parse().ok()).unwrap_or(7878);
    let runtime = tokio::runtime::Runtime::new()?;
    runtime.block_on(async { easyssh_core::debug_ws::run_server(&host, port).await })?;
    Ok(())
}

fn print_help() {
    println!("EasySSH Core CLI");
    println!();
    println!("Commands:");
    println!("  easyssh add-server <name> <host> <username> [port] [auth]");
    println!("  easyssh add-group <name>");
    println!("  easyssh list");
    println!("  easyssh import-ssh");
    println!("  easyssh connect <server-id>");
    println!("  easyssh debug-server [host] [port]");
}

fn validate_server(
    name: &str,
    host: &str,
    username: &str,
    port: &str,
    auth_type: &str,
) -> Result<(), Box<dyn Error>> {
    if name.is_empty() {
        return Err("服务器名称不能为空".into());
    }
    if host.is_empty() {
        return Err("主机地址不能为空".into());
    }
    if username.is_empty() {
        return Err("用户名不能为空".into());
    }
    let port_num: u16 = port.parse()?;
    if port_num == 0 {
        return Err("端口号不能为0".into());
    }
    match auth_type {
        "password" | "key" | "agent" => Ok(()),
        _ => Err("无效的认证类型".into()),
    }
}

fn validate_group_name(name: &str) -> Result<(), Box<dyn Error>> {
    if name.is_empty() {
        return Err("分组名称不能为空".into());
    }
    Ok(())
}
