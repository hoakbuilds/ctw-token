export type CtwToken = {
  version: "0.1.0";
  name: "ctw_token";
  instructions: [
    {
      name: "initialize";
      docs: [
        "Initialize a Confidential Transfer enabled Token Extensions Mint for an existing SPL Token Mint.",
        "This Confidential Transfer enabled Token Extensions Mint, or Confidential Wrapped Token Mint,",
        "effectively represents the same underlying SPL Token but with the ability to use Token Extensions'",
        "zk-powered confidential transfers which mask the amount being transferred.",
        "",
        "# Notes",
        "",
        "This implementation does not require any new CT-enabled Token Accounts to be approved and",
        "are 1:1 equivalents of the SPL Token."
      ];
      accounts: [
        {
          name: "tokenMint";
          isMut: false;
          isSigner: false;
          docs: [
            "The SPL Token Mint for which we want to create a Confidential Transfers Mint Wrapper."
          ];
        },
        {
          name: "confidentialMint";
          isMut: true;
          isSigner: false;
          docs: ["The SPL Token Extensions Mint."];
        },
        {
          name: "programAuthority";
          isMut: false;
          isSigner: false;
          docs: ["The authority of the Confidential Wrapper Token Program."];
        },
        {
          name: "tokenVault";
          isMut: true;
          isSigner: false;
          docs: ["The token vault."];
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
          docs: ["The fee and rent payer."];
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Program."];
        },
        {
          name: "associatedTokenProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Associated Token Program."];
        },
        {
          name: "tokenExtensionsProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Extensions Program."];
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
          docs: ["The System Program."];
        }
      ];
      args: [
        {
          name: "auditorPubkey";
          type: {
            array: ["u8", 32];
          };
        }
      ];
    },
    {
      name: "wrap";
      docs: [
        "Wrap the given token amount of an SPL Token into an equivalent amount of a Confidential Wrapped Token Mint.",
        "",
        "# Notes",
        "",
        "The integrator is responsible for passing in a TokenAccount for the `confidential_token_account` param",
        "that has already been initialized and for which the [`ConfigureAccount`] as well as, if necessary,",
        "the [`ApproveAccount`] instructions have been executed.",
        "",
        "After this instruction is called, the integrator is then free to call [`Deposit`] and [`ApplyPendingBalance`]",
        "in order to roll the token amount into the available balance of the Confidential Token Account."
      ];
      accounts: [
        {
          name: "tokenMint";
          isMut: false;
          isSigner: false;
          docs: ["The mint of the token being wrapped."];
        },
        {
          name: "tokenAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "confidentialMint";
          isMut: true;
          isSigner: false;
          docs: ["The mint of the token being wrapped."];
        },
        {
          name: "confidentialTokenAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "programAuthority";
          isMut: false;
          isSigner: false;
          docs: ["The authority of the Confidential Wrapper Token Program."];
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
          docs: ["The authority of the source token account."];
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
          docs: ["The fee and rent payer."];
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Interface."];
        },
        {
          name: "tokenExtensionsProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Interface."];
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    },
    {
      name: "unwrap";
      docs: [
        "Unwrap the given token amount of a Confidential Wrapped Token back into it's corresponding",
        "SPL Token Mint.",
        "",
        "# Notes",
        "",
        "The integrator is responsible for assuring that the user has enough non-confidential",
        "balance in order to unwrap and redeem for the underlying token.",
        "This can be achieved by having the [`Withdraw`] instruction being successfully executed beforehand."
      ];
      accounts: [
        {
          name: "tokenMint";
          isMut: false;
          isSigner: false;
          docs: ["The mint of the token being wrapped."];
        },
        {
          name: "tokenAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenVault";
          isMut: true;
          isSigner: false;
        },
        {
          name: "confidentialMint";
          isMut: true;
          isSigner: false;
          docs: ["The mint of the token being wrapped."];
        },
        {
          name: "confidentialTokenAccount";
          isMut: true;
          isSigner: false;
        },
        {
          name: "programAuthority";
          isMut: false;
          isSigner: false;
          docs: ["The authority of the Confidential Wrapper Token Program."];
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
          docs: ["The authority of the source token account."];
        },
        {
          name: "payer";
          isMut: true;
          isSigner: true;
          docs: ["The fee and rent payer."];
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Interface."];
        },
        {
          name: "tokenExtensionsProgram";
          isMut: false;
          isSigner: false;
          docs: ["The Token Interface."];
        }
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        }
      ];
    }
  ];
};

export const IDL: CtwToken = {
  version: "0.1.0",
  name: "ctw_token",
  instructions: [
    {
      name: "initialize",
      docs: [
        "Initialize a Confidential Transfer enabled Token Extensions Mint for an existing SPL Token Mint.",
        "This Confidential Transfer enabled Token Extensions Mint, or Confidential Wrapped Token Mint,",
        "effectively represents the same underlying SPL Token but with the ability to use Token Extensions'",
        "zk-powered confidential transfers which mask the amount being transferred.",
        "",
        "# Notes",
        "",
        "This implementation does not require any new CT-enabled Token Accounts to be approved and",
        "are 1:1 equivalents of the SPL Token.",
      ],
      accounts: [
        {
          name: "tokenMint",
          isMut: false,
          isSigner: false,
          docs: [
            "The SPL Token Mint for which we want to create a Confidential Transfers Mint Wrapper.",
          ],
        },
        {
          name: "confidentialMint",
          isMut: true,
          isSigner: false,
          docs: ["The SPL Token Extensions Mint."],
        },
        {
          name: "programAuthority",
          isMut: false,
          isSigner: false,
          docs: ["The authority of the Confidential Wrapper Token Program."],
        },
        {
          name: "tokenVault",
          isMut: true,
          isSigner: false,
          docs: ["The token vault."],
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
          docs: ["The fee and rent payer."],
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Program."],
        },
        {
          name: "associatedTokenProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Associated Token Program."],
        },
        {
          name: "tokenExtensionsProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Extensions Program."],
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
          docs: ["The System Program."],
        },
      ],
      args: [
        {
          name: "auditorPubkey",
          type: {
            array: ["u8", 32],
          },
        },
      ],
    },
    {
      name: "wrap",
      docs: [
        "Wrap the given token amount of an SPL Token into an equivalent amount of a Confidential Wrapped Token Mint.",
        "",
        "# Notes",
        "",
        "The integrator is responsible for passing in a TokenAccount for the `confidential_token_account` param",
        "that has already been initialized and for which the [`ConfigureAccount`] as well as, if necessary,",
        "the [`ApproveAccount`] instructions have been executed.",
        "",
        "After this instruction is called, the integrator is then free to call [`Deposit`] and [`ApplyPendingBalance`]",
        "in order to roll the token amount into the available balance of the Confidential Token Account.",
      ],
      accounts: [
        {
          name: "tokenMint",
          isMut: false,
          isSigner: false,
          docs: ["The mint of the token being wrapped."],
        },
        {
          name: "tokenAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "confidentialMint",
          isMut: true,
          isSigner: false,
          docs: ["The mint of the token being wrapped."],
        },
        {
          name: "confidentialTokenAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "programAuthority",
          isMut: false,
          isSigner: false,
          docs: ["The authority of the Confidential Wrapper Token Program."],
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
          docs: ["The authority of the source token account."],
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
          docs: ["The fee and rent payer."],
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Interface."],
        },
        {
          name: "tokenExtensionsProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Interface."],
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "unwrap",
      docs: [
        "Unwrap the given token amount of a Confidential Wrapped Token back into it's corresponding",
        "SPL Token Mint.",
        "",
        "# Notes",
        "",
        "The integrator is responsible for assuring that the user has enough non-confidential",
        "balance in order to unwrap and redeem for the underlying token.",
        "This can be achieved by having the [`Withdraw`] instruction being successfully executed beforehand.",
      ],
      accounts: [
        {
          name: "tokenMint",
          isMut: false,
          isSigner: false,
          docs: ["The mint of the token being wrapped."],
        },
        {
          name: "tokenAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenVault",
          isMut: true,
          isSigner: false,
        },
        {
          name: "confidentialMint",
          isMut: true,
          isSigner: false,
          docs: ["The mint of the token being wrapped."],
        },
        {
          name: "confidentialTokenAccount",
          isMut: true,
          isSigner: false,
        },
        {
          name: "programAuthority",
          isMut: false,
          isSigner: false,
          docs: ["The authority of the Confidential Wrapper Token Program."],
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
          docs: ["The authority of the source token account."],
        },
        {
          name: "payer",
          isMut: true,
          isSigner: true,
          docs: ["The fee and rent payer."],
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Interface."],
        },
        {
          name: "tokenExtensionsProgram",
          isMut: false,
          isSigner: false,
          docs: ["The Token Interface."],
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
  ],
};
