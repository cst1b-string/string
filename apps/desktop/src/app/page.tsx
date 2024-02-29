"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import SignUp from "@/components/signUp";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

var hasAccount = false;

export default function Home() {
	if (!hasAccount) {
		return (
			< SignUp />
		)
	}
	const [lightMode, setLightMode] = useState(false);

	return (
		<themeContext.Provider value={{ lightMode, setLightMode }}>
			<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
				<div className="">
					<ChatSidebar />
				</div>

				<div className="col-span-2 text-white font-bold ">
					<ChatLog />
				</div>
			</div>
		</themeContext.Provider>
	);
}
