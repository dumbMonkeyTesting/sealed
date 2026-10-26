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

`app/index.html` talks to devnet from the browser. Open it over https, not as a file on disk. The browser blocks the library imports from a `file://` page. Once the repo is on GitHub, this preview works:

`https://htmlpreview.github.io/?https://github.com/OWNER/REPO/blob/BRANCH/app/index.html`

Paste the program id and run. The page creates throwaway wallets, shields 0.05 SOL, and sends 0.02 SOL to a second wallet. Proof generation takes a while. A successful run ends with the second wallet holding exactly 0.02 SOL.

Devnet can reset, and the public faucet rate-limits airdrops. A failed run after the vault exists needs a fresh deploy, because `initialize` can only run once per program id.
