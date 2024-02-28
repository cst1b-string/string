"use client";

import { IntegrationProvider } from "@/integration";
import type { Metadata } from "next";
import { Inter } from "next/font/google";

import { Navbar } from "../components/navbar";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<IntegrationProvider>
			<html lang="en">
				<body className={inter.className}>
					<Navbar />
					{children}
				</body>
			</html>
		</IntegrationProvider>
	);
}
