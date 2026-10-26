# Sealed

A devnet program that locks SOL in a vault and mints the same number of lamports as cSOL. cSOL is a Token-2022 token with confidential balances. Sending cSOL encrypts the amount. Shielding and unshielding stay public.

The pump.fun coin is not this mint. cSOL is only a receipt for SOL sitting in the vault.

## Playground

Nothing in this repo has been compiled. Playground compiles it.

1. Open https://beta.solpg.io/ and create the Playground wallet. Leave it on devnet. Use that wallet, not Phantom.
2. In the Playground terminal run `solana airdrop 5`. If that is rate-limited, paste the wallet address into https://faucet.solana.com.
3. Create a new Anchor project named `sealed`.
4. Replace that project's `src/lib.rs` with `programs/sealed/src/lib.rs` from this folder.
5. In the terminal run `build`, then `deploy`.
6. Copy the program id Playground writes into `declare_id!`.

Playground can also import a GitHub folder. After this repo is on GitHub, the import link is:

`https://beta.solpg.io/https://github.com/OWNER/REPO/tree/BRANCH/programs/sealed`

## Test

`app/index.html` talks to devnet from the browser. From the `app` folder, run `python -m http.server 8765` and open `http://127.0.0.1:8765/`. Opening the file directly, or through an HTML preview site, does not run it.

Paste the program id and run. The page creates new wallets, shields 0.03 SOL, and sends that 0.03 SOL to a second wallet. It does not read a keypair file and it does not spend the Playground wallet. Proof generation takes a while. A successful run ends with the second wallet holding exactly 0.03 SOL.

Devnet can reset, and the public faucet rate-limits airdrops. A failed run after the vault exists needs a fresh deploy, because `initialize` can only run once per program id.
