import { createContext, useContext } from "react";
import type { Source } from "./types.ts";

export const SourceContext = createContext<Source | null>(null);

export function useSource(): Source {
  const s = useContext(SourceContext);
  if (!s) throw new Error("no source connected");
  return s;
}
