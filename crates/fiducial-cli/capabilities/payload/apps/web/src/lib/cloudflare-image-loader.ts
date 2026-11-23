type ImageLoaderProps = {
  src: string;
  width: number;
  quality?: number;
};

export default function cloudflareImageLoader({ src, width, quality }: ImageLoaderProps): string {
  const params = [`width=${width}`, "format=auto", "fit=scale-down"];
  if (quality) params.push(`quality=${quality}`);
  return `/cdn-cgi/image/${params.join(",")}/${src}`;
}
