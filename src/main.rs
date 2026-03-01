//! MCP Reasoning Server binary entry point.
//!
//! This binary provides a stdio-based MCP server for structured reasoning.
//! All logs go to stderr; stdout is reserved for MCP JSON-RPC messages.
//!
//! Coverage is excluded because the main function cannot be unit tested
//! as it requires the full MCP protocol handshake over stdio.

// Enable the coverage attribute when running with nightly for llvm-cov exclusions
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use mcp_reasoning::anthropic::AnthropicClient;
use mcp_reasoning::config::Config;
use mcp_reasoning::server::McpServer;
use mcp_reasoning::storage::SqliteStorage;

#[cfg_attr(coverage_nightly, coverage(off))]
#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                print_version();
                std::process::exit(0);
            }
            "--health" => {
                if let Err(e) = run_health_check().await {
                    eprintln!("Health check failed: {e}");
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[1]);
                eprintln!();
                print_help();
                std::process::exit(1);
            }
        }
    }

    // Initialize logging to stderr only (stdout is for MCP JSON-RPC)
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("LOG_LEVEL")
                .unwrap_or_else(|_| "info".to_string())
                .parse()
                .unwrap_or_else(|_| tracing_subscriber::filter::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("mcp-reasoning starting...");

    // Load configuration from environment
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        "Configuration loaded: database={}, timeout={}ms",
        config.database_path,
        config.request_timeout_ms
    );

    // Create and run server
    let server = McpServer::new(config);
    if let Err(e) = server.run_stdio().await {
        tracing::error!("Server error: {e}");
        std::process::exit(1);
    }

    tracing::info!("mcp-reasoning shutdown complete");
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn print_version() {
    println!("mcp-reasoning {}", env!("CARGO_PKG_VERSION"));
    println!("Rust MCP server for structured reasoning");
    println!();
    println!("15 reasoning tools:");
    println!("  Core: linear, tree, divergent, reflection, checkpoint, auto");
    println!("  Graph: graph (8 operations)");
    println!("  Analysis: detect, decision, evidence");
    println!("  Advanced: timeline, mcts, counterfactual");
    println!("  Infrastructure: preset, metrics");
    println!();
    println!("Repository: https://github.com/quanticsoul4772/mcp-reasoning");
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn print_help() {
    println!("MCP Reasoning Server v{}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE:");
    println!("    mcp-reasoning [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --version, -v    Print version information and exit");
    println!("    --health         Run health checks and exit");
    println!("    --help, -h       Print this help message and exit");
    println!();
    println!("    (no arguments)   Start MCP server on stdio");
    println!();
    println!("ENVIRONMENT VARIABLES:");
    println!("    ANTHROPIC_API_KEY       Anthropic API key (required)");
    println!("    DATABASE_PATH           SQLite database path (default: ./data/reasoning.db)");
    println!("    LOG_LEVEL               Log level: error|warn|info|debug|trace (default: info)");
    println!("    REQUEST_TIMEOUT_MS      Request timeout in milliseconds (default: 30000)");
    println!("    MAX_RETRIES             Maximum API retry attempts (default: 3)");
    println!();
    println!("EXAMPLES:");
    println!("    # Run server");
    println!("    export ANTHROPIC_API_KEY=sk-ant-xxx");
    println!("    mcp-reasoning");
    println!();
    println!("    # Check version");
    println!("    mcp-reasoning --version");
    println!();
    println!("    # Run health checks");
    println!("    mcp-reasoning --health");
    println!();
    println!("DOCUMENTATION:");
    println!("    https://github.com/quanticsoul4772/mcp-reasoning");
}

#[cfg_attr(coverage_nightly, coverage(off))]
async fn run_health_check() -> Result<(), Box<dyn std::error::Error>> {
    println!("🏥 MCP Reasoning Server - Health Check");
    println!("========================================");
    println!();

    // Check 1: API key
    print!("1. API key configured... ");
    if std::env::var("ANTHROPIC_API_KEY").is_ok() {
        println!("✅");
    } else {
        println!("❌");
        println!();
        println!("Error: ANTHROPIC_API_KEY environment variable not set");
        println!("Get your API key from: https://console.anthropic.com/");
        println!();
        println!("Set it with:");
        println!("  export ANTHROPIC_API_KEY=sk-ant-xxx  # Unix/macOS");
        println!("  set ANTHROPIC_API_KEY=sk-ant-xxx     # Windows");
        return Err("Missing API key".into());
    }

    // Check 2: Configuration
    print!("2. Configuration valid... ");
    let config = match Config::from_env() {
        Ok(config) => {
            println!("✅");
            config
        }
        Err(e) => {
            println!("❌");
            println!();
            println!("Error: {e}");
            return Err(e.into());
        }
    };

    // Check 3: Database connectivity
    print!("3. Database connection... ");
    match SqliteStorage::new(&config.database_path).await {
        Ok(_) => println!("✅"),
        Err(e) => {
            println!("❌");
            println!();
            println!(
                "Error: Cannot connect to database at {}",
                config.database_path
            );
            println!("Details: {e}");
            println!();
            println!("Ensure the directory exists:");
            println!("  mkdir -p data  # Unix/macOS");
            println!("  mkdir data     # Windows");
            return Err(e.into());
        }
    }

    // Check 4: API client creation
    print!("4. API client initialization... ");
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    match AnthropicClient::with_api_key(api_key) {
        Ok(_) => println!("✅"),
        Err(e) => {
            println!("❌");
            println!();
            println!("Error: Cannot create Anthropic API client");
            println!("Details: {e}");
            return Err(Box::new(e));
        }
    }

    println!();
    println!("✅ All health checks passed!");
    println!();
    println!("Server is ready to use.");
    println!("Run without arguments to start: mcp-reasoning");

    Ok(())
}
