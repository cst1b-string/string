"use client";

import { IntegrationProvider } from "@/integration";
import { Inter } from "next/font/google";
import React, { createContext } from "react";

import { LoginProvider } from "../components/contexts/loginContext";
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

const WithTheme: React.FC<React.PropsWithChildren<{}>> = ({ children }) => {
	const rspc = useRspc();
	const { data } = rspc.useQuery(["settings.theme"]);
	return (
		<html lang="en" className={`${data === "Light" ? "" : "dark"}`}>
			<body className={inter.className}>
				<Navbar />
				{children}
			</body>
		</html>
	);
};

export default function RootLayout({
	children,
}: Readonly<{
	children: React.ReactNode;
}>) {
	return (
		<IntegrationProvider>
			<LoginProvider>
				<ThemeContext.Provider value={{ lightMode, setLightMode }}>
					<html lang="en">
						<body className={inter.className}>
							<Navbar />
							{children}
						</body>
					</html>
				</ThemeContext.Provider>
			</LoginProvider>
		</IntegrationProvider>
	);
}
