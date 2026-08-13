import { createServer } from "node:http";

const port = 4318;

const users = [
  { id: "usr_1042", name: "Анна Смирнова", role: "admin", status: "active" },
  { id: "usr_1048", name: "Максим Орлов", role: "developer", status: "active" },
  { id: "usr_1055", name: "Елена Волкова", role: "analyst", status: "invited" },
];

const server = createServer((request, response) => {
  response.setHeader("Access-Control-Allow-Origin", "*");
  response.setHeader("Content-Type", "application/json; charset=utf-8");
  response.setHeader("X-Request-Id", "demo_01J5RV9D7QJ4K87H");
  response.setHeader("X-RateLimit-Remaining", "98");

  if (request.url?.startsWith("/v1/users")) {
    response.writeHead(200);
    response.end(JSON.stringify({ data: users, meta: { total: users.length, page: 1, next_cursor: null } }));
    return;
  }
  if (request.url === "/v1/health") {
    response.writeHead(200);
    response.end(JSON.stringify({ status: "ok", version: "2026.08", services: { database: "up", queue: "up" } }));
    return;
  }
  response.writeHead(404);
  response.end(JSON.stringify({ error: { code: "not_found", message: "Route not found" } }));
});

server.listen(port, "127.0.0.1", () => {
  console.log(`ReqVault showcase API: http://127.0.0.1:${port}`);
});
