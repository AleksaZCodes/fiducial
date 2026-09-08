import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title:       "{{name}}",
  description: "Built on Fiducial {{version}}",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
