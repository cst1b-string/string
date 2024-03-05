"use client";

import { IntegrationProvider, useRspc } from "@/integration";
import { Inter } from "next/font/google";
import { createContext, useState } from "react";

import { Navbar } from "../components/navbar";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

interface ThemeState {
	lightMode: boolean;
	setLightMode: (value: boolean) => void;
}

export const ThemeContext = createContext<ThemeState>({
	lightMode: false,
	setLightMode: (value: boolean) => {
		throw new Error("Context used without provider");
	},
});

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	const [lightMode, setLightMode] = useState(false);

	return (
		<IntegrationProvider>
			<ThemeContext.Provider value={{ lightMode, setLightMode }}>
				<html lang="en">
					<body className={inter.className}>
						<Navbar />
						{children}
					</body>
				</html>
			</ThemeContext.Provider>
		</IntegrationProvider>
	);
}
