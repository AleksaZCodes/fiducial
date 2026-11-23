import "@payloadcms/next/css";
import { handleServerFunctions, RootLayout } from "@payloadcms/next/layouts";
import config from "@payload-config";
import type { ServerFunctionClient } from "payload";
import type React from "react";
import { importMap } from "./importMap.js";

export default async function Layout({ children }: { children: React.ReactNode }) {
  const serverFunction: ServerFunctionClient = async (args) => {
    "use server";
    return handleServerFunctions({ ...args, config, importMap });
  };

  return (
    <RootLayout config={config} importMap={importMap} serverFunction={serverFunction}>
      {children}
    </RootLayout>
  );
}
