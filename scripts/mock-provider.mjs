import http from "node:http";

const port = Number(process.env.YIYU_MOCK_PROVIDER_PORT ?? 11434);
const model = process.env.YIYU_MOCK_PROVIDER_MODEL ?? "qwen3:8b";
const chunks = ["译羽", "已经", "准备好", "了。"];

const server = http.createServer((request, response) => {
  if (request.method === "GET" && request.url === "/v1/models") {
    response.writeHead(200, { "content-type": "application/json" });
    response.end(JSON.stringify({ data: [{ id: model }] }));
    return;
  }

  if (request.method === "POST" && request.url === "/v1/chat/completions") {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk) => {
      body += chunk;
    });
    request.on("end", () => {
      let stream = true;
      try {
        stream = JSON.parse(body).stream !== false;
      } catch {
        response.writeHead(400, { "content-type": "application/json" });
        response.end(JSON.stringify({ error: { message: "invalid JSON body" } }));
        return;
      }

      if (!stream) {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(
          JSON.stringify({
            choices: [{ message: { content: chunks.join("") } }],
          }),
        );
        return;
      }

      response.writeHead(200, {
        "content-type": "text/event-stream",
        "cache-control": "no-cache",
        connection: "keep-alive",
      });
      let index = 0;
      const timer = setInterval(() => {
        if (index < chunks.length) {
          response.write(
            `data: ${JSON.stringify({
              choices: [{ delta: { content: chunks[index] } }],
            })}\n\n`,
          );
          index += 1;
          return;
        }
        clearInterval(timer);
        response.end("data: [DONE]\n\n");
      }, 80);
      response.on("close", () => clearInterval(timer));
    });
    return;
  }

  response.writeHead(404, { "content-type": "application/json" });
  response.end(JSON.stringify({ error: { message: "not found" } }));
});

server.listen(port, "127.0.0.1", () => {
  console.log(`Yiyu mock provider listening on http://127.0.0.1:${port}/v1`);
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => server.close(() => process.exit(0)));
}
