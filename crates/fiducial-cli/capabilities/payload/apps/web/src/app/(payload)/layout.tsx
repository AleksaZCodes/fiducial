import "@payloadcms/next/css";
import type { ServerSideHtmlProps } from "@payloadcms/next/layouts";
import { RootLayout } from "@payloadcms/next/layouts";
import config from "@payload-config";

export default async function Layout({ children, params }: ServerSideHtmlProps) {
  return (
    <RootLayout config={config} params={params}>
      {children}
    </RootLayout>
  );
}
