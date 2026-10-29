import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Cloudflare Tunnel Control | Auto-Provisioner Dashboard",
  description: "Deploy and manage Cloudflare Tunnels and ingress routes across machines with a single curl command.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <body
        className="min-h-screen bg-neutral-950 text-neutral-100 antialiased selection:bg-orange-500/30 selection:text-orange-200"
        suppressHydrationWarning
      >
        {children}
      </body>
    </html>
  );
}
