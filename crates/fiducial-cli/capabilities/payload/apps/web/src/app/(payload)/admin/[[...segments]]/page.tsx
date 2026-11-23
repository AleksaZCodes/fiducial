import { generatePageMetadata, RootPage } from "@payloadcms/next/views";
import config from "@payload-config";
import { importMap } from "../../importMap.js";

type Props = {
  params: Promise<{ segments: string[] }>;
  searchParams: Promise<Record<string, string>>;
};

export default function Page({ params, searchParams }: Props) {
  return <RootPage config={config} importMap={importMap} params={params} searchParams={searchParams} />;
}

export async function generateMetadata({ params, searchParams }: Props) {
  return generatePageMetadata({ config, params, searchParams });
}
