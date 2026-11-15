import type { Route } from "./+types/home";

export function meta(_: Route.MetaArgs) {
  return [{ title: "{{name}}" }];
}

// A loader runs on the Worker and its result reaches the component typed, with no
// hand-written API in between.
export function loader() {
  return { message: "Hello from the Worker" };
}

export default function Home({ loaderData }: Route.ComponentProps) {
  return (
    <main>
      <h1>{{name}}</h1>
      <p>{loaderData.message}</p>
    </main>
  );
}
