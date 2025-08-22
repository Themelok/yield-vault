
import { getReservesForMarket } from "klend-sdk";
import { address, Address, Rpc } from '@solana/kit';


const connection = new Rpc("https://api.mainnet-beta.solana.com");

// Kamino Main market (public from SDK examples)
const MAIN_MARKET = address("7u3HeHxYDLhnCoErrtycNokbQYbWGzLs6JSDqGAv5PfF");

(async () => {
  const reserves = await getReservesForMarket(MAIN_MARKET, connection, "mainnet", 1000);
  
  // Find USDC reserve by symbol
  let usdcReserve;
  for (const [_, reserve] of reserves) {
    if (reserve.config.tokenInfo.symbol === "USDC") {
      usdcReserve = reserve;
      break;
    }
  }
  
  if (!usdcReserve) throw new Error("USDC reserve not found");
  
  // This is what you need:
  const collateralMint = usdcReserve.config.collateralMint;
  console.log("Kamino USDC collateral mint:", collateralMint.toBase58());
})();