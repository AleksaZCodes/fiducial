import { registerHooks } from "node:module";
import { existsSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

registerHooks({
  resolve(specifier, context, nextResolve) {
    if (
      !context.parentURL?.startsWith("file://") ||
      (!specifier.startsWith("./") && !specifier.startsWith("../"))
    ) {
      return nextResolve(specifier, context);
    }
    const base = fileURLToPath(new URL(specifier, context.parentURL));
    for (const ext of [".ts", ".tsx", "/index.ts"]) {
      if (existsSync(base + ext)) {
        return { url: pathToFileURL(base + ext).href, shortCircuit: true };
      }
    }
    return nextResolve(specifier, context);
  },
});
