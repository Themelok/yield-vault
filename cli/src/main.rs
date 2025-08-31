use clap::{Parser, Subcommand};
use anyhow::{anyhow, Result};
use solana_sdk::{
    signature::{read_keypair_file}, signer::Signer,
};
mod commands;
mod consts;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
#[command(name = "vault-cli", version, about = "CLI for Yield Vault")]
enum Command {
    Init{keypair_path: std::path::PathBuf,},
    Deposit {
        #[arg(short, long)]
        amount: u64,
    },
    Withdraw {
        #[arg(short, long)]
        amount: u64,
    },
}

fn main() -> Result<()> {
    let args = Cli::parse();


    match args.cmd {
        Command::Init { keypair_path } => {
            commands::init(keypair_path)?;
        }
        Command::Deposit { amount } => {
            println!("Deposit {}", amount);
   
        }
        Command::Withdraw { amount } => {
            println!("Withdraw {}", amount);
       
        }
    
    }
    Ok(())
}

// let content = std::fs::read_to_string(&args.keypair_path).with_context(|| format!("could not read file `{}`", args.keypair_path.display()))?;

 
