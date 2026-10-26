import fs from "node:fs";
import {
  address,
  appendTransactionMessageInstruction,
  createKeyPairSignerFromBytes,
  createSolanaRpc,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signTransactionMessageWithSigners,
} from "@solana/kit";
import { getTransferSolInstruction } from "@solana-program/system";

const EXPECTED_SOURCE = "5HEfWKbNVoniEuMwqZ9KEom2RLWhkJDEqCAWiZnuwXfU";
const AMOUNT = 100_000_000n;
const destinationText = process.argv[2];
if (!destinationText) {
  console.error("Pass the test wallet address.");
  process.exit(1);
}

const keypairPath = new URL("../wallet-1-keypair-sealed.json", import.meta.url);
const raw = JSON.parse(fs.readFileSync(keypairPath, "utf8"));
if (!Array.isArray(raw) || raw.length !== 64) {
  throw new Error("Keypair file is not a 64-byte Solana keypair.");
}
const signer = await createKeyPairSignerFromBytes(Uint8Array.from(raw));
if (signer.address !== EXPECTED_SOURCE) {
  throw new Error("Keypair file is not the Playground wallet.");
}
if (destinationText === EXPECTED_SOURCE) {
  throw new Error("Refusing to send the test amount back to the Playground wallet.");
}

const rpc = createSolanaRpc("https://api.devnet.solana.com");
const destination = address(destinationText);
const before = await rpc.getBalance(signer.address).send();
console.log("FROM=" + signer.address);
console.log("TO=" + destination);
console.log("BEFORE=" + before.value.toString());
if (before.value < AMOUNT + 10_000n) {
  throw new Error("Playground wallet does not have 0.1 SOL plus a fee.");
}

const latest = await rpc.getLatestBlockhash().send();
const instruction = getTransferSolInstruction({
  source: signer,
  destination,
  amount: AMOUNT,
});
const message = pipe(
  createTransactionMessage({ version: 0 }),
  (tx) => setTransactionMessageFeePayerSigner(signer, tx),
  (tx) => setTransactionMessageLifetimeUsingBlockhash(latest.value, tx),
  (tx) => appendTransactionMessageInstruction(instruction, tx),
);
const signed = await signTransactionMessageWithSigners(message);
const signature = await rpc.sendTransaction(getBase64EncodedWireTransaction(signed), {
  encoding: "base64",
  preflightCommitment: "confirmed",
}).send();
console.log("SIGNATURE=" + signature);

for (let attempt = 0; attempt < 30; attempt += 1) {
  const status = await rpc.getSignatureStatuses([signature]).send();
  const info = status.value[0];
  if (info && (info.confirmationStatus === "confirmed" || info.confirmationStatus === "finalized")) {
    if (info.err) throw new Error("Transfer failed: " + JSON.stringify(info.err));
    break;
  }
  await new Promise((resolve) => setTimeout(resolve, 1000));
}

const after = await rpc.getBalance(signer.address).send();
const received = await rpc.getBalance(destination).send();
console.log("AFTER_SOURCE=" + after.value.toString());
console.log("DEST_BALANCE=" + received.value.toString());
