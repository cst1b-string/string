"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

export default function Home() {
	const [lightMode, setLightMode] = useState(false);

	return (
		<themeContext.Provider value={{ lightMode, setLightMode }}>
			<div className="flex divide-x divide-gray-400 h-[calc(100vh-80px)]">
				<div>
					<ChatSidebar />
				</div>

				<div className="text-white font-bold flex-grow">
					<ChatLog />
					{/* send messages */}
					<div className="px-4">
						<input
							type="text"
							className="w-90p h-10 px-6 rounded-xl bg-lightGrey font-normal"
							placeholder="message here"
						/>
					</div>
				</div>
			</div>
		</themeContext.Provider>
	);
}
