"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import Textarea from "@mui/joy/Textarea";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

export default function Home() {
	const [lightMode, setLightMode] = useState(false);

	return (
		<themeContext.Provider value={{ lightMode, setLightMode }}>
			<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
				<div className="">
					<ChatSidebar />
				</div>

				<div className="col-span-2 text-white font-bold ">
					<ChatLog />
					<div style={{ padding: "10px 0", position: "relative", left: "1%", width: "95%" }}>
						<Textarea
							placeholder="Type a message here..."
							color="neutral"
							maxRows={3}
							variant="soft"
							className="input"
						/>
					</div>
				</div>
			</div>
		</themeContext.Provider>
	);
}
