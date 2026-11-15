import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";

// A server function runs on the Worker and is called from the page like any
// function, typed end to end, with no API route written by hand.
const hello = createServerFn({ method: "GET" }).handler(async () => {
  return { message: "Hello from the Worker" };
});

export const Route = createFileRoute("/")({
  loader: () => hello(),
  component: Home,
});

function Home() {
  const { message } = Route.useLoaderData();
  return (
    <main>
      <h1>{{name}}</h1>
      <p>{message}</p>
    </main>
  );
}
