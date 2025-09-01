use clap::{Parser, Subcommand};
use anyhow::{Result};
mod commands;
mod consts;
mod http_client;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
#[command(name = "vault-cli", version, about = "CLI for Yield Vault")]
enum Command {
    Init {
        keypair_path: std::path::PathBuf
    },
    Deposit {
        #[arg(short, long)]
        amount: u64,

        keypair_path: std::path::PathBuf,
    },
    Withdraw {
        keypair_path: std::path::PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.cmd {
        Command::Init { keypair_path } => {
            commands::init(keypair_path)?;
        }
        Command::Deposit { keypair_path, amount } => {
            println!("Deposit {}", amount);
            commands::deposit(keypair_path, amount)?;
   
        }
        Command::Withdraw { keypair_path } => {
            println!("Withdraw");
            commands::withdraw(keypair_path)?;
        }
    
    }
    Ok(())
}


 
