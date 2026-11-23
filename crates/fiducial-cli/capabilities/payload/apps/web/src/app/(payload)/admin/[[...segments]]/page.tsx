import { RootPage, generatePageMetadata } from "@payloadcms/next/layouts";
import config from "@payload-config";

type Props = {
  params: Promise<{ segments: string[] }>;
  searchParams: Promise<Record<string, string>>;
};

export default async function Page({ params, searchParams }: Props) {
  return <RootPage config={config} params={params} searchParams={searchParams} />;
}

export async function generateMetadata({ params, searchParams }: Props) {
  return generatePageMetadata({ config, params, searchParams });
}
