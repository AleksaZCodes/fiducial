import type { Result } from "@fiducial/headless";

// Declare the product state as a typed Result — headless logic, no framework coupling.
const status: Result<string, string> = { ok: true, value: "{{name}}" };

// The platform version, written in a string rather than inline in the JSX
// below: a formatter reads a double-braced placeholder in JSX text as an
// object literal and re-spaces it, after which `fid` no longer expands it.
const platformVersion = "{{version}}";

export default function Home() {
  return (
    <main style={{ padding: "2rem", maxWidth: "640px", margin: "0 auto" }}>
      <h1 style={{ color: "var(--primary)", marginBottom: "0.5rem" }}>
        {status.ok ? status.value : "Error"}
      </h1>
      <p style={{ color: "var(--muted-foreground)" }}>
        Built on{" "}
        <a href="https://github.com/AleksaZCodes/fiducial" style={{ color: "var(--primary)" }}>
          Fiducial
        </a>{" "}
        {platformVersion}.
      </p>
      {!status.ok && <p style={{ color: "var(--destructive)" }}>{status.error}</p>}
    </main>
  );
}
