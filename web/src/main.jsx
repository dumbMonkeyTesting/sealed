import React, { useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { PrivyProvider, useConnectWallet } from "@privy-io/react-auth";
import { toSolanaWalletConnectors, useStandardWallets } from "@privy-io/react-auth/solana";
import { createSolanaRpc, createSolanaRpcSubscriptions } from "@solana/kit";

const site = window.SEALED || {};
const connectors = toSolanaWalletConnectors({ shouldAutoConnect: false });

function wantedName(wallet) {
  const label = String(wallet.walletClientType || wallet.connectorType || wallet.name || "").toLowerCase();
  if (label.includes("backpack")) return "Backpack";
  if (label.includes("phantom")) return "Phantom";
  return "";
}

async function resolveStandard(list, connected) {
  const name = wantedName(connected);
  const pool = list.filter((wallet) => wallet.name === "Phantom" || wallet.name === "Backpack");
  const named = pool.find((wallet) => wallet.name === name) || pool.find((wallet) => (wallet.accounts || []).some((account) => account.address === connected.address));
  const standard = named || pool[0];
  if (!standard || !standard.features || !standard.features["solana:signTransaction"]) {
    throw new Error("Phantom or Backpack did not finish connecting. Install one of them and try again.");
  }
  let accounts = standard.accounts || [];
  if (!accounts.length && standard.features["standard:connect"]) {
    const result = await standard.features["standard:connect"].connect();
    accounts = result.accounts || [];
  }
  const account = accounts.find((item) => item.address === connected.address)
    || accounts.find((item) => item.features && item.features.includes("solana:signTransaction"))
    || accounts[0];
  if (!account) throw new Error("The wallet did not return an account.");
  return { wallet: standard, account };
}

function Bridge() {
  const walletsRef = useRef([]);
  const { wallets } = useStandardWallets();
  walletsRef.current = wallets;
  const { connectWallet } = useConnectWallet({
    onSuccess: async ({ wallet }) => {
      try {
        const name = wantedName(wallet);
        let list = walletsRef.current;
        for (let attempt = 0; attempt < 20; attempt += 1) {
          const named = list.find((item) => item.name === name);
          const accounts = (named && named.accounts) || [];
          if (wallet.address && accounts.some((account) => account.address === wallet.address)) break;
          if (!name && list.some((item) => item.name === "Phantom" || item.name === "Backpack")) break;
          await new Promise((resolve) => setTimeout(resolve, 100));
          list = walletsRef.current;
        }
        const connected = await resolveStandard(list, wallet);
        window.dispatchEvent(new CustomEvent("sealed-connected", { detail: connected }));
      } catch (error) {
        window.dispatchEvent(new CustomEvent("sealed-connect-error", { detail: error.message || String(error) }));
      }
    },
    onError: (code) => {
      const message = code === "exited_auth_flow" ? "Wallet connection was cancelled." : String(code || "Could not connect the wallet.");
      window.dispatchEvent(new CustomEvent("sealed-connect-error", { detail: message }));
    },
  });

  useEffect(() => {
    window.sealedConnect = () => new Promise((resolve, reject) => {
      const ok = (event) => {
        cleanup();
        resolve(event.detail);
      };
      const err = (event) => {
        cleanup();
        reject(new Error(event.detail || "Could not connect the wallet."));
      };
      function cleanup() {
        window.removeEventListener("sealed-connected", ok);
        window.removeEventListener("sealed-connect-error", err);
      }
      window.addEventListener("sealed-connected", ok);
      window.addEventListener("sealed-connect-error", err);
      connectWallet({
        walletList: ["phantom", "backpack"],
        walletChainType: "solana-only",
        description: "Connect Phantom or Backpack",
      });
    });
    return () => {
      delete window.sealedConnect;
    };
  }, [connectWallet]);

  return null;
}

const root = document.getElementById("privy-root");
if (!site.privyAppId) {
  window.sealedConnect = () => Promise.reject(new Error("Add privyAppId in deployment.js. Create the app in the Privy dashboard and allow this site's address."));
} else if (root) {
  const chain = site.cluster === "mainnet" || site.cluster === "mainnet-beta" ? "solana:mainnet" : "solana:" + (site.cluster || "devnet");
  const rpc = site.rpc || "https://api.devnet.solana.com";
  const rpcSubscriptions = site.rpcSubscriptions || "wss://api.devnet.solana.com";
  createRoot(root).render(
    <PrivyProvider
      appId={site.privyAppId}
      clientId={site.privyClientId || undefined}
      config={{
        loginMethods: ["wallet"],
        appearance: {
          theme: "#14110e",
          accentColor: "#E4B15A",
          walletChainType: "solana-only",
          walletList: ["phantom", "backpack"],
          showWalletLoginFirst: true,
        },
        externalWallets: {
          solana: { connectors },
        },
        embeddedWallets: {
          solana: { createOnLogin: "off" },
        },
        solana: {
          rpcs: {
            [chain]: {
              rpc: createSolanaRpc(rpc),
              rpcSubscriptions: createSolanaRpcSubscriptions(rpcSubscriptions),
            },
          },
        },
      }}
    >
      <Bridge />
    </PrivyProvider>,
  );
}
