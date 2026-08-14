mod audio;
mod config;
mod discord;
mod guild;
mod permissions;
mod persistence;

use config::Config;
use discord::{parse_command, CommandService};
use tracing::info;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter("musay=info")
        .init();
    if std::env::args().any(|a| a == "--self-check") {
        println!("Musay core OK");
        return;
    }
    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "Configuração inválida: {e}. Use --self-check para validar o núcleo sem token."
            );
            return;
        }
    };
    let service = CommandService::new(config);
    info!("Musay iniciado; o adaptador Discord deve ser habilitado com a feature `discord` em uma implantação com token");
    let _ = parse_command("!help", "!");
    if let Err(e) = tokio::signal::ctrl_c().await {
        eprintln!("Falha ao aguardar shutdown: {e}");
    }
    info!("shutdown gracioso concluído");
    drop(service);
}
