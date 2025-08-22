

# **An Implementation Blueprint for a Solana-Based Automated USDC Yield Aggregator**

This report provides a comprehensive, two-phase implementation plan for a sophisticated automated USDC yield aggregator on the Solana blockchain. The primary objective is to engineer a system that maximizes returns for users by intelligently allocating USDC deposits between two of Solana's leading lending protocols: Kamino Finance and MarginFi. The initial phase will focus on building a robust on-chain and off-chain foundation with Kamino as the sole yield source. The second phase will integrate MarginFi to enable the core functionality of the system: dynamic, automated yield switching based on real-time market data. This document is intended to serve as the definitive architectural and engineering guide for the project, providing a clear and actionable roadmap for development, security, and deployment.

## **Section 1: System Architecture and Core Components**

This section defines the high-level architecture of the aggregator, establishing the relationship between its on-chain and off-chain components. A clear understanding of this structure is fundamental to appreciating the system's security model, operational flow, and scalability.

### **1.1. Architectural Overview: The Vault-Keeper Model**

The system will be architected using a dual-component model, a proven and robust pattern for DeFi automation protocols. This model separates the core logic of fund management from the decision-making intelligence, enhancing both security and flexibility.

* **On-Chain Vault Program:** A Solana program, developed in Rust using the Anchor framework, will serve as the decentralized custodian of all user funds. This program is the execution layer, responsible for all on-chain financial operations. Its logic is transparent, immutable once deployed, and publicly verifiable on the Solana blockchain. It is the single source of truth for asset management.  
* **Off-Chain Keeper Application:** A Rust application will operate as the system's autonomous intelligence layer. This keeper is responsible for monitoring market conditions, executing the yield-optimization algorithm, and triggering on-chain transactions when profitable opportunities are identified.

This separation of concerns is a deliberate architectural choice that aligns with DeFi security best practices. It places the critical, value-bearing logic on-chain, where it is subject to the security guarantees of the Solana network. The more complex, computationally intensive, and potentially fallible decision-making logic is handled off-chain. The keeper is granted a limited set of permissions, allowing it only to invoke specific, predefined instructions on the on-chain vault, thereby minimizing its potential attack surface and preventing it from ever gaining direct control over user assets.

### **1.2. The On-Chain Vault Program: The Execution Layer**

The on-chain vault program is the heart of the aggregator. It is a set of smart contracts that directly manages user deposits and interacts with the underlying lending protocols.

* **Functionality:** The vault program's primary responsibilities are to:  
  1. Accept USDC deposits from users.  
  2. Mint and issue a corresponding amount of share tokens (e.g., vyUSDC, for "Vault Yield USDC") to depositors. These tokens represent a user's proportional ownership of the total assets managed by the vault.  
  3. Process user withdrawals by accepting and burning vyUSDC share tokens in exchange for the underlying USDC.  
  4. Execute Cross-Program Invocations (CPIs) to deposit assets into and withdraw assets from the integrated lending protocols (initially Kamino, later MarginFi).1  
* **Key Characteristics:** The program will be fundamentally non-custodial; the development team will have no administrative keys or special privileges that would allow for the movement of user funds. Its behavior is governed entirely by its code. An administrative role will be designated for initial parameter setup (e.g., setting the keeper's authorized address), but the design will include a clear pathway for this authority to be transitioned to a more decentralized governance structure, such as a multisig wallet or a DAO, as the protocol matures.

### **1.3. The Off-Chain Keeper Application: The Intelligence Layer**

The off-chain keeper application provides the automation that makes the aggregator "smart." It runs on secure, private infrastructure and acts as a servant to the on-chain vault, executing its strategy based on external data.

* **Functionality:** The keeper's core responsibilities include:  
  1. **Data Aggregation:** Periodically fetching APY data for USDC lending from all integrated protocols. This involves querying off-chain REST APIs where available (e.g., Kamino's API) and making direct RPC calls to the Solana network to read on-chain program states (e.g., for MarginFi).2  
  2. **Strategic Decision-Making:** Executing the core yield-switching algorithm. This logic compares the net APYs of the available protocols to determine the optimal allocation of the vault's capital.  
  3. **Profitability Analysis:** Calculating the expected profit of a potential rebalance. This is a critical step that must account for all associated costs, including Solana network transaction fees, compute unit priority fees, and any potential slippage, to ensure that a rebalance is only executed when it is economically advantageous.  
  4. **Transaction Execution:** Constructing, signing, and broadcasting the rebalance transaction to the on-chain vault program when a profitable yield-switching opportunity has been confirmed.  
* **Operational Requirements:** The keeper application must be designed for high availability and fault tolerance. It should include robust error handling, retry mechanisms for transient network issues, and comprehensive logging and alerting to notify operators of any anomalies.

### **1.4. Data and Control Flow**

The interaction between the user, the vault, the keeper, and the underlying protocols follows a clear and logical sequence for each primary operation.

* **User Deposit Flow:**  
  1. A user initiates a deposit transaction targeting the on-chain vault program.  
  2. The vault program receives the user's USDC.  
  3. The vault program calculates the number of *vyUSDC* share tokens to mint based on the current total assets under management and the total supply of shares.  
  4. The vault program executes a CPI to deposit the received USDC into the currently active lending protocol (e.g., Kamino).  
  5. The vault program transfers the newly minted *vyUSDC* share tokens to the user.  
* **Keeper-Initiated Rebalance Flow:**  
  1. The off-chain keeper fetches the latest APY from Kamino and MarginFi.  
  2. The keeper's algorithm determines that MarginFi's APY is now significantly higher than Kamino's, exceeding the profitability threshold.  
  3. The keeper constructs and signs a rebalance transaction, invoking the corresponding instruction on the on-chain vault program.  
  4. The vault program, upon receiving the authorized rebalance call, executes a CPI to withdraw the entire USDC balance from the Kamino lending pool.  
  5. Immediately within the same atomic transaction, the vault program executes a second CPI to deposit the withdrawn USDC into the MarginFi lending pool.  
  6. The vault program updates its internal state to reflect that MarginFi is now the active protocol.  
* **User Withdrawal Flow:**  
  1. A user initiates a withdraw transaction, sending their vyUSDC share tokens to the vault program.  
  2. The vault program burns the user's share tokens.  
  3. The program calculates the proportional amount of USDC the user is entitled to based on the number of shares burned.  
  4. The vault program executes a CPI to withdraw the required amount of USDC from the active lending protocol.  
  5. The vault program transfers the USDC to the user's wallet.

This architectural model ensures that all value-transfer operations are executed atomically on-chain, while the strategic impetus for those operations is provided by a dedicated, specialized off-chain service.

## **Section 2: Phase 1: Foundational Implementation with Kamino Finance**

The first phase of development is dedicated to building the core infrastructure of the aggregator and integrating with Kamino Finance as the initial and sole source of yield. This phased approach mitigates risk by allowing the team to focus on establishing a stable and secure foundation before introducing the additional complexity of multi-protocol logic.

### **2.1. On-Chain Program: Initial Build (Anchor/Rust)**

The on-chain vault will be developed using the Anchor framework, which significantly streamlines Solana program development in Rust by providing a more structured and secure environment.4

* **Environment Setup:** The project will be initialized using the Anchor CLI (anchor init). The Cargo.toml file will be configured to include essential dependencies: anchor-lang for the core framework, anchor-spl for interacting with SPL tokens like USDC, and a dedicated Kamino CPI crate to facilitate interaction with the Kamino Lend program.  
* **Program State Definition:** The program's on-chain state will be managed through Program Derived Addresses (PDAs) to ensure the program itself can sign for its accounts.  
  * VaultState PDA: This singleton account will store the global configuration for the vault. Its fields will include the public key of the administrative authority, the public key of the keeper, the mint address of the USDC token, the mint address of the vault's vyUSDC share token, and the address of the Kamino USDC lending reserve.  
* **Core Instructions:**  
  * initialize\_vault: An administrative instruction, callable only once by the designated admin authority. This function will be responsible for creating the VaultState PDA, creating the vyUSDC share token mint, and setting the initial configuration parameters.  
  * deposit\_usdc: A public instruction that enables users to deposit USDC. The core logic involves a cross-program invocation to the SPL Token program to transfer USDC from the user to a vault-controlled token account. It then calculates the appropriate number of vyUSDC shares to mint based on the formula: shares\_to\_mint \= (deposit\_amount \* total\_shares) / total\_assets. Finally, it executes a CPI to deposit the USDC into the Kamino Lend program.  
  * withdraw\_usdc: A public instruction allowing users to redeem their vyUSDC shares. The logic first calculates the amount of USDC to be returned: usdc\_to\_return \= (shares\_burned \* total\_assets) / total\_shares. It then executes a CPI to withdraw this specific amount of USDC from Kamino back to the vault, followed by an SPL Token transfer to the user's wallet.  
* **Kamino CPI Integration:**  
  * **Program ID Resolution:** A critical prerequisite for development is the definitive resolution of two conflicting Program IDs for Kamino Lend found in official sources. The official documentation lists GzFgdRJXmawPhGeBsyRCDLx4jAKPsvbUqoqitzppkzkW 5, whereas the  
    klend source code repository specifies KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD for Mainnet.6 Further complicating this, the documentation for the Kamino V2 upgrade states that the V1 codebase was preserved, which implies the Program ID should not have changed, eliminating the need for a user fund migration.7 This discrepancy likely arises from documentation referring to a broader "Lend Program" while the GitHub repository contains the specific, deployable  
    klend program. The most reliable method for verification involves inspecting recent, successful deposit transactions initiated via the official Kamino Finance user interface on a Solana explorer like Solscan. By analyzing the instruction logs of these transactions, the true program address being invoked for USDC lending can be definitively identified.8 For initial development purposes, the address from the source code repository,  
    KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD, will be used provisionally.  
  * **CPI Implementation:** The integration will leverage a pre-existing Rust CPI crate such as kamino\_lending\_interface or kamino\_lend.10 These crates, which are generated for Anchor programs, are invaluable as they abstract away the complexities of manual instruction data serialization and provide strongly-typed structs for defining the accounts required by the target instruction. This aligns with the best practices for CPIs outlined in the official Solana documentation.1 The choice to build our vault with Anchor is heavily influenced by the availability of these tools from Kamino, as it creates a consistent and less error-prone development environment.  
  * **Required Accounts:** A successful CPI requires a meticulously ordered list of accounts. The TypeScript examples in the klend-sdk repository can be reverse-engineered to determine the exact accounts needed for the deposit and withdraw instructions in the Rust context.12 This list will include the user's wallet, the source and destination token accounts, the vault's PDA authority (which will sign the CPI), the specific Kamino market and reserve accounts for USDC, and the SPL Token Program itself. These will be defined within a dedicated Anchor  
    Accounts struct for maximum type safety.

### **2.2. Off-Chain Keeper: Monitoring and Execution (Rust)**

The Phase 1 keeper will be a simpler version of its final form, focusing on data collection and system monitoring without performing any rebalancing actions.

* **Project Setup:** A new Rust binary project will be created using Cargo. The primary dependencies will include solana-client for all RPC communications with the Solana cluster, solana-sdk for transaction building and signing utilities, reqwest for making asynchronous HTTP requests to external APIs, serde and serde\_json for deserializing API responses, and tokio as the asynchronous runtime.  
* **Configuration Management:** All sensitive and environment-specific parameters will be managed in a config.toml file and loaded at runtime. This includes the Solana RPC endpoint URL, the on-chain vault program ID, and the file path to the keeper's wallet keypair. The keeper's private key must never be hardcoded in the source code.  
* **APY Fetching Module:** This module will be responsible for querying Kamino's public API to retrieve yield data for the USDC lending pool. It will make a GET request to the documented endpoint, such as https://api.kamino.finance/strategies or a more specific lending endpoint.2 The module must include robust error handling to manage potential network failures, API rate limiting, or changes in the JSON response structure.  
* **Transaction Orchestration:** In Phase 1, the keeper's on-chain interaction will be limited to read-only calls. It will be programmed to periodically fetch and deserialize the VaultState account from the blockchain. This serves as a health check, confirming that the on-chain program is deployed and its state is readable. This lays the groundwork for the transaction-sending logic that will be implemented in Phase 2\.

## **Section 3: Phase 2: Integration of MarginFi and Automated Yield Switching**

This phase introduces the full functionality of the aggregator. It builds upon the stable foundation from Phase 1 to incorporate MarginFi as a second yield venue and implements the core decision-making logic that drives automated capital allocation.

### **3.1. On-Chain Program: Advanced Capabilities**

The on-chain vault program will be upgraded to manage multiple protocols and execute the rebalancing of funds between them.

* **State Augmentation:** The VaultState PDA struct will be extended to support the new functionality:  
  * current\_protocol: An enum (enum Protocol { Kamino, MarginFi }) will be added to track the protocol where the vault's funds are currently deposited. This acts as a state machine, informing the rebalance logic.  
  * marginfi\_market\_address: A Pubkey field will be added to store the address of the MarginFi bank for USDC.  
* **The rebalance Instruction:** This is the most critical and sensitive instruction in the program.  
  * **Authority:** Access to this instruction must be strictly controlled. It will be protected with an Anchor constraint (\#\[account(constraint \=...)\]) to ensure it can only be successfully invoked when the transaction is signed by the authorized keeper wallet address stored in VaultState.  
  * **Logic:** The instruction will accept the target protocol as an input parameter. Its core function is to execute an atomic withdrawal of the vault's entire USDC balance from the current\_protocol and a subsequent deposit into the target protocol. This operation must occur within a single transaction to prevent a state where funds are withdrawn but not redeposited, which would leave capital idle and unproductive. The sequence of CPIs will be:  
    1. Invoke the withdraw instruction on the source protocol's program (e.g., Kamino's deposit\_reserve\_liquidity\_and\_obligation\_collateral).  
    2. Invoke the deposit instruction on the target protocol's program (e.g., MarginFi's lending\_account\_deposit).  
* **MarginFi CPI Integration:**  
  * **Program ID:** The Mainnet Program ID for MarginFi v2 is consistently identified across its repositories and documentation as MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA.13  
  * **CPI Implementation:** The marginfi-cpi crate will be added as a dependency to the project's Cargo.toml.15 This crate, similar to the one for Kamino, provides the necessary Rust functions and typed structs to correctly build CPIs to MarginFi's core instructions, specifically  
    lending\_account\_deposit and lending\_account\_withdraw, which are detailed in the protocol's program-level documentation.16  
  * **Required Accounts:** The MarginFi documentation provides a clear specification of the accounts required for its lending instructions.14 This includes the  
    marginfi\_group, the marginfi\_account (which will be our vault's PDA), the specific USDC bank, and various associated token accounts. These will be defined in a dedicated Anchor accounts struct for the rebalance instruction to ensure compile-time safety and clarity.

To ensure the on-chain program is both maintainable and extensible, the architecture should abstract the protocol-specific logic. A naive implementation might embed Kamino- and MarginFi-specific calls within a large conditional block in the rebalance instruction. A more robust and forward-looking approach involves defining a generic LendingAdaptor interface, or Rust trait. This allows for the creation of modular, protocol-specific implementations (kamino\_adaptor.rs, marginfi\_adaptor.rs) that conform to a standard interface. This design pattern significantly simplifies the core rebalancing logic and streamlines the future integration of additional lending protocols.

| Parameter | Kamino Finance | MarginFi |  |
| :---- | :---- | :---- | :---- |
| **Mainnet Program ID** | KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD (Verified from source repo 6, pending on-chain tx confirmation) | MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA (Verified from source repo 13) |  |
| **Rust CPI Crate** | kamino\_lend 11 or | kamino\_lending\_interface 10 | marginfi-cpi 15 |
| **Primary APY Source** | REST API: api.kamino.finance 2 | On-Chain RPC: Query Bank account state |  |
| **Fallback APY Source** | On-Chain RPC: Query Reserve account state | N/A |  |
| **USDC Pool Address** | To be determined via SDK/on-chain analysis | To be determined via SDK/on-chain analysis |  |

### **3.2. Off-Chain Keeper: The Decision Engine**

The keeper evolves into its final form in Phase 2, equipped with the logic to make intelligent, data-driven decisions about capital allocation.

* **Multi-Protocol APY Fetching:** The keeper's data aggregation module will be expanded to source data from both protocols.  
  * It will continue to query the Kamino API as its primary data source for Kamino's APY.2  
  * A new method will be implemented to determine MarginFi's USDC lending APY. As the research does not indicate a dedicated public REST API for this purpose, the primary method will be to fetch the MarginFi Bank account for USDC directly from the blockchain via an RPC call. The APY can then be calculated using the interest rate parameters and utilization figures stored within the deserialized account data. The structure of this data can be inferred from the MarginFi TypeScript SDK documentation.3  
  * This dual-sourcing strategy presents a potential architectural imbalance. Relying on an off-chain API for one protocol and on-chain data for another introduces a single point of failure if the Kamino API becomes unavailable or reports stale information. To mitigate this, the keeper must be engineered with a fallback mechanism. It should have the capability to calculate Kamino's APY directly from its on-chain Reserve account state, using the API as a faster, primary source but always having the on-chain data as a ground truth for verification and resilience.  
* **Comparative Analysis and Profitability Heuristics:** This is the core of the keeper's intelligence.  
  * The logic will continuously compare the net APYs: apy\_kamino versus apy\_marginfi.  
  * A rebalance will only be triggered if the difference in yield exceeds a configurable threshold: (apy\_target \- apy\_current) \> threshold.  
  * This threshold is a critical parameter. It must be carefully calibrated to be greater than the total cost of executing the rebalance transaction. This includes the base network fee, any priority fees required for timely inclusion in a block, and a small buffer to prevent "thrashing"—a scenario where the keeper performs frequent, low-profit rebalances that erode gains.  
* **Triggering Mechanism:** Once the keeper identifies a profitable rebalancing opportunity that satisfies the threshold condition, it will execute the following sequence:  
  1. Construct the rebalance instruction for the on-chain vault program, specifying the target protocol.  
  2. Build a Solana transaction containing this single instruction.  
  3. Sign the transaction using its securely stored keypair.  
  4. Send the transaction to the Solana cluster using the sendAndConfirmTransaction RPC method, with appropriate retry logic.  
  5. Implement comprehensive logging for every stage of this process and configure alerts for both successful and, more importantly, failed rebalance attempts.

## **Section 4: Security, Deployment, and Operational Readiness**

Moving the aggregator from a development environment to a secure, production-ready state on Mainnet requires a rigorous focus on security, a structured deployment process, and robust operational monitoring.

### **4.1. Security Posture and Auditing**

Security is the paramount concern for any protocol handling user funds. The following measures are non-negotiable.

* **Input Validation:** All instructions that accept account inputs from a user must perform rigorous validation checks. This includes ensuring that provided token mints match the expected USDC mint stored in VaultState and that account ownership is as expected.  
* **Authority Checks:** Every privileged instruction—initialize\_vault, rebalance, set\_new\_admin, etc.—must be protected by Anchor's \#\[account(constraint \=...)\] macro. This declarative approach ensures that the Solana runtime will reject any attempt to call these functions unless the transaction is signed by the correct authority key.  
* **Secure CPI Practices:** When making CPIs, the program must use Anchor's CpiContext::new\_with\_signer() method. This correctly passes the vault's PDA "signer seeds," allowing the vault program to sign for the CPI. This prevents common vulnerabilities where a malicious actor could trick the program into signing with an unintended authority.  
* **Third-Party Audit:** Before any user funds are accepted on Mainnet, the on-chain program's codebase must undergo a full, independent security audit by a reputable firm specializing in Solana smart contract security. The public repository of audits maintained by Kamino Finance serves as an example of the industry standard for transparency and diligence in this area.17

### **4.2. Deployment and Configuration**

A methodical deployment strategy across multiple environments is essential to ensure a smooth and secure launch.

* **Deployment Workflow:** The Anchor CLI will be used for building and deploying the on-chain program (anchor build, anchor deploy). The build process should follow the verifiable build guidelines to produce a deterministic binary, allowing the community to verify that the deployed on-chain code exactly matches the public source code.18  
* **Environment Strategy:**  
  1. **Devnet:** The initial deployment target for functional testing, integration testing of the vault and keeper, and debugging.  
  2. **Testnet:** Used for a final round of end-to-end testing in an environment that more closely mirrors Mainnet conditions.  
  3. **Mainnet-beta:** The final production deployment.  
* **Keeper Deployment:** The off-chain keeper application should be deployed on a secure, high-availability cloud infrastructure provider (e.g., AWS, GCP). The keeper's wallet private key is a highly sensitive asset and must be managed using a dedicated secret management service (e.g., AWS Secrets Manager, HashiCorp Vault), never stored in plaintext configuration files or environment variables.

### **4.3. Monitoring and Maintenance**

Continuous monitoring of both on-chain and off-chain components is critical for maintaining the health and security of the protocol.

* **On-Chain Monitoring:** Dashboards should be created using tools like Solana Explorer, Dune Analytics, or custom solutions to track key performance indicators (KPIs) in real-time. This includes Total Value Locked (TVL), transaction volume, the vault's current USDC balance in each underlying protocol, and the vyUSDC share price.  
* **Keeper Monitoring:** The keeper application must implement structured logging (e.g., JSON format) for all significant actions (APY fetches, decision-making, transaction submissions). These logs should be ingested into a centralized logging platform (e.g., Datadog, Grafana Loki). Alerts should be configured for critical error conditions, such as repeated transaction failures, API downtime from Kamino, significant RPC node latency, or unexpected APY volatility. The operational principles of production-grade off-chain bots, such as the public Kamino liquidator, can serve as a model for building a resilient keeper.19  
* **Community and Support:** To foster a healthy developer ecosystem around the protocol, a dedicated support channel should be established. Following the best practice seen in projects like Kamino, a \#dev-support channel on Discord would provide a venue for developers to ask questions and get assistance when integrating with the aggregator.19

## **Conclusion**

This implementation plan outlines a robust, secure, and phased approach to building an automated USDC yield aggregator on Solana. By prioritizing a foundational build with Kamino Finance in Phase 1, the project can de-risk the development of core on-chain and off-chain components. The subsequent integration of MarginFi and the activation of the automated rebalancing logic in Phase 2 will deliver the full value proposition of the system.

The architectural decision to use the Vault-Keeper model, combined with the choice of the Anchor framework for on-chain development, aligns with industry best practices for security and maintainability. Key challenges, such as the resolution of Kamino's Program ID and the development of a resilient APY data-sourcing strategy, have been identified with clear action plans. Adherence to the stringent security protocols, a methodical deployment strategy, and the implementation of comprehensive monitoring are critical for the long-term success and trustworthiness of the protocol. By following this blueprint, the development team can efficiently engineer a sophisticated and capital-efficient DeFi product for the Solana ecosystem.

## **Appendix**

### **On-Chain Instruction Account Requirements for rebalance**

The following table details every account required by the rebalance instruction. This serves as a definitive checklist for developers implementing the Anchor Accounts struct.

| Account Name | Writable | Signer | PDA/Source | Description |
| :---- | :---- | :---- | :---- | :---- |
| keeper | No | Yes | Keeper's Wallet | The authorized off-chain entity calling the instruction. |
| vault\_state | Yes | No | PDA | The global state account for the aggregator vault. |
| vault\_authority | No | No | PDA | The PDA that serves as the authority for the vault's token accounts and CPIs. |
| vault\_usdc\_token\_account | Yes | No | PDA | The vault's token account holding USDC temporarily during rebalance. |
| source\_protocol\_program | No | No | Static | The Program ID of the protocol to withdraw from (e.g., Kamino). |
| source\_protocol\_market | Yes | No | Static | The market/group account for the source protocol. |
| source\_protocol\_reserve | Yes | No | Static | The reserve/bank account for USDC in the source protocol. |
| ... (other source accounts) | Varies | No | Varies | Additional accounts required by the source protocol's withdraw CPI. |
| destination\_protocol\_program | No | No | Static | The Program ID of the protocol to deposit into (e.g., MarginFi). |
| destination\_protocol\_market | Yes | No | Static | The market/group account for the destination protocol. |
| destination\_protocol\_reserve | Yes | No | Static | The reserve/bank account for USDC in the destination protocol. |
| ... (other destination accounts) | Varies | No | Varies | Additional accounts required by the destination protocol's deposit CPI. |
| spl\_token\_program | No | No | Static | The official SPL Token Program address. |

#### **Works cited**

1. Cross Program Invocation (CPI) \- Solana, accessed August 19, 2025, [https://solana.com/docs/core/cpi](https://solana.com/docs/core/cpi)  
2. Kamino-Finance/kamino-api-docs \- GitHub, accessed August 19, 2025, [https://github.com/Kamino-Finance/kamino-api-docs](https://github.com/Kamino-Finance/kamino-api-docs)  
3. TypeScript SDK \- marginfi Documentation, accessed August 19, 2025, [https://docs.marginfi.com/ts-sdk](https://docs.marginfi.com/ts-sdk)  
4. kamino\_lending\_interface \- crates.io: Rust Package Registry, accessed August 19, 2025, [https://crates.io/crates/kamino\_lending\_interface](https://crates.io/crates/kamino_lending_interface)  
5. SDK & Smart Contracts \- Kamino Docs, accessed August 19, 2025, [https://docs.kamino.finance/build-on-kamino/sdk-and-smart-contracts](https://docs.kamino.finance/build-on-kamino/sdk-and-smart-contracts)  
6. Kamino-Finance/klend \- GitHub, accessed August 19, 2025, [https://github.com/Kamino-Finance/klend](https://github.com/Kamino-Finance/klend)  
7. Kamino Launches V2, Ushering In A New Era Of Modular Credit ..., accessed August 19, 2025, [https://www.rockawayx.com/insights/kamino-launches-v2-ushering-in-a-new-era-of-modular-credit-infrastructure-on-solana](https://www.rockawayx.com/insights/kamino-launches-v2-ushering-in-a-new-era-of-modular-credit-infrastructure-on-solana)  
8. Create Transactions on the Solana Network, accessed August 19, 2025, [https://solana.com/developers/courses/intro-to-solana/intro-to-writing-data](https://solana.com/developers/courses/intro-to-solana/intro-to-writing-data)  
9. Transactions and Instructions | Solana, accessed August 19, 2025, [https://solana.com/docs/core/transactions](https://solana.com/docs/core/transactions)  
10. kamino\_lending\_interface \- crates.io: Rust Package Registry, accessed August 19, 2025, [https://crates.io/crates/kamino\_lending\_interface/versions](https://crates.io/crates/kamino_lending_interface/versions)  
11. kamino\_lend \- Rust \- Docs.rs, accessed August 19, 2025, [https://docs.rs/kamino-lend/latest/kamino\_lend/](https://docs.rs/kamino-lend/latest/kamino_lend/)  
12. Kamino-Finance/klend-sdk: 🛠️ Kamino Lending TypeScript SDK \- GitHub, accessed August 19, 2025, [https://github.com/Kamino-Finance/klend-sdk](https://github.com/Kamino-Finance/klend-sdk)  
13. marginfi-v2/scripts/verify.sh at main \- GitHub, accessed August 19, 2025, [https://github.com/mrgnlabs/marginfi-v2/blob/main/scripts/verify.sh](https://github.com/mrgnlabs/marginfi-v2/blob/main/scripts/verify.sh)  
14. marginfi documentation, accessed August 19, 2025, [https://docs.marginfi.com/](https://docs.marginfi.com/)  
15. marginfi-cpi \- crates.io: Rust Package Registry, accessed August 19, 2025, [https://crates.io/crates/marginfi-cpi](https://crates.io/crates/marginfi-cpi)  
16. marginfi v2 Program Documentation, accessed August 19, 2025, [https://docs.marginfi.com/mfi-v2](https://docs.marginfi.com/mfi-v2)  
17. Kamino Finance \- GitHub, accessed August 19, 2025, [https://github.com/Kamino-Finance](https://github.com/Kamino-Finance)  
18. How to Verify a Program \- Solana, accessed August 19, 2025, [https://solana.com/developers/guides/advanced/verified-builds](https://solana.com/developers/guides/advanced/verified-builds)  
19. Kamino-Finance/liquidator-public: open source version of a liquidation bot running against kamino-lend \- GitHub, accessed August 19, 2025, [https://github.com/Kamino-Finance/liquidator-public](https://github.com/Kamino-Finance/liquidator-public)