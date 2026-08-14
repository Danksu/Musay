use musay::config::Config;
use tracing::info;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter("musay=info")
        .init();

    if std::env::args().any(|arg| arg == "--self-check") {
        match Config::for_self_check() {
            Ok(_) => println!("Musay core OK"),
            Err(error) => {
                eprintln!("self-check falhou: {error}");
                std::process::exit(2);
            }
        }
        return;
    }

    #[cfg(feature = "discord")]
    {
        let token = match rpassword::prompt_password("Token do bot Discord (entrada oculta): ") {
            Ok(token) => token,
            Err(error) => {
                eprintln!("não foi possível ler o token: {error}");
                std::process::exit(2);
            }
        };
        let config = match Config::from_token(token) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("token/configuração inválida: {error}");
                std::process::exit(2);
            }
        };
        info!("iniciando conexão com o Discord; pressione Ctrl+C para encerrar");
        if let Err(error) = musay::discord::runtime::BotRuntime::new(config).run().await {
            eprintln!("o bot encerrou com erro: {error}");
            std::process::exit(1);
        }
        info!("bot encerrado de forma graciosa");
    }

    #[cfg(not(feature = "discord"))]
    {
        eprintln!("esta build foi compilada sem a feature `discord`; execute `cargo run` com as features padrão");
        std::process::exit(2);
    }
}
