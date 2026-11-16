import { Form, data, useNavigation } from "react-router";
import { greetingFor } from "../lib/greeting.server";
import type { Route } from "./+types/home";

export function meta(_: Route.MetaArgs) {
  return [{ title: "{{name}}" }];
}

// A loader runs on the Worker for the GET and reaches the component typed.
export function loader() {
  return { heading: "{{name}}" };
}

// An action runs on the Worker for the POST. The form below works with
// JavaScript off (a plain POST and a rendered response) and, once the page has
// hydrated, without a full reload. That is the full-stack loop: a typed loader to
// read, an action to write, and a server-only module behind both.
export async function action({ request }: Route.ActionArgs) {
  const form = await request.formData();
  const name = String(form.get("name") ?? "").trim();

  if (!name) {
    return data({ error: "Tell us your name.", greeting: null }, { status: 400 });
  }
  if (name.length > 80) {
    return data({ error: "That name is too long.", greeting: null }, { status: 400 });
  }
  return { error: null, greeting: greetingFor(name) };
}

export default function Home({ loaderData, actionData }: Route.ComponentProps) {
  const navigation = useNavigation();
  const sending = navigation.state === "submitting";

  return (
    <main>
      <h1>{loaderData.heading}</h1>
      <Form method="post">
        <label htmlFor="name">Your name</label>{" "}
        <input id="name" name="name" autoComplete="name" required maxLength={80} />{" "}
        <button type="submit" disabled={sending}>
          {sending ? "Sending…" : "Say hello"}
        </button>
      </Form>
      {actionData?.error ? <p role="alert">{actionData.error}</p> : null}
      {actionData?.greeting ? <p>{actionData.greeting}</p> : null}
    </main>
  );
}
