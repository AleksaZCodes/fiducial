// The editor is a local tool, not a route of the site: nothing here is
// deployed, and there is no login, because the way in is a checked-out
// repository. See the `cms` skill.
export const metadata = { title: "Content", robots: "noindex" };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
