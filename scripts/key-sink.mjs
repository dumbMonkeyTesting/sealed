import fs from "node:fs";
import http from "node:http";

const dir = new URL("../test-keys/", import.meta.url);
fs.mkdirSync(dir, { recursive: true });
const file = new URL("wallets.jsonl", dir);

function isByte(value) {
  return Number.isInteger(value) && value >= 0 && value <= 255;
}

const server = http.createServer((request, response) => {
  response.setHeader("Access-Control-Allow-Origin", "*");
  response.setHeader("Access-Control-Allow-Headers", "content-type");
  response.setHeader("Access-Control-Allow-Methods", "POST, OPTIONS");
  if (request.method === "OPTIONS") {
    response.writeHead(204);
    response.end();
    return;
  }
  if (request.method !== "POST" || request.url !== "/keys") {
    response.writeHead(404);
    response.end();
    return;
  }
  const chunks = [];
  request.on("data", (chunk) => chunks.push(chunk));
  request.on("end", () => {
    try {
      const body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      if (typeof body.role !== "string" || typeof body.address !== "string") {
        throw new Error("missing role or address");
      }
      if (!Array.isArray(body.secret) || body.secret.length !== 32 || !body.secret.every(isByte)) {
        throw new Error("secret must be 32 bytes");
      }
      fs.appendFileSync(file, JSON.stringify({
        role: body.role,
        address: body.address,
        secret: body.secret,
        savedAt: new Date().toISOString(),
      }) + "\n");
      response.writeHead(204);
      response.end();
    } catch (error) {
      response.writeHead(400);
      response.end(error.message);
    }
  });
});

server.listen(8766, "127.0.0.1", () => {
  console.log("Saving test wallet keys on 127.0.0.1:8766");
});
