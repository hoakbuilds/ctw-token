# Confidential Transfer Wrapped Token Program (CTW Token)

This repository contains a Solana program built with the new SPL Token Extensions, the goal is to give users the ability to perform Confidential Transfers on SPL Token Mints OR SPL Token Extensions Mints which do not have the Confidential Transfers Extension, along with a CLI and a small TS SDK to help potential integrations.

## Program

The program features only three instructions:

- Initialize
  - This permissionless instruction allows creating a CTW Mint for any given SPL Token Mint
  - All CTW Token Mints have the same number of decimals as their SPL Token Mint counterpart
  - The freeze authority of the existing SPL Token Mint is COPIED over to the CTW Mint, meaning if it is set it will also be set on the new Mint
- Wrap
  - This instruction allows wrapping a given amount of an SPL Token Mint OR SPL Token Extensions Mint for the corresponding amount of the equivalent CTW Mint
  - An initialized and configured Confidential Transfer Account (CTA) must be passed in
  - The given amount of SPL Token is transferred from the user's Legacy Token Account into the program's vault and an equivalent amount of the CTW Token is minted into the public component of the CTA and instantly deposited
  - Integrators still need to execute `ApplyPendingBalance` after calling this instruction
- Unwrap
  - This instruction allows unwrapping a given amount of a CTW Mint for the corresponding amount of the equivalent SPL Token Mint
  - A CTA with enough balance in it's public component must be passed in
  - The given amount of CTW Token is burned by the program and an equivalent amount of the SPL Token Mint is transferred from the program's vault into the user's Legacy Token Account
  - Integrators may need to execute `Withdraw` and `VerifyWithdraw` beforehand to guarantee the previous point

## Notes

- The program is currently unable to be used in any of the clusters due to `zk-token-proof` not being present.
- In order to successfully test the program using `solana-program-test`, the SPL Token Extensions Program had to be built locally with the `zk-ops` feature enabled and the output was used to override the `spl_token_2022` program available.