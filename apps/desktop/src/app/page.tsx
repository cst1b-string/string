"use client";

import ChatLog from "@/components/chatLog";
import ChatSidebar from "@/components/chatSidebar";
import { useRspc } from "@/integration";
import { redirect } from "next/navigation";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

export default function Home() {
	const [selectedChannel, setSelectedChannel] = useState(-1);
	const rspc = useRspc();
	const hasAccount = rspc.useQuery(["account.login", null]).data;

	if (!hasAccount) {
		console.log("in main page");
		redirect("/signUp");
	}

	const [lightMode, setLightMode] = useState(false);

	return (
		<div className="grid grid-cols-[auto,1fr] h-[calc(100vh-60px)]">
			<ChatSidebar selectedChannel={selectedChannel} setSelectedChannel={setSelectedChannel} />
			<div className="grid grid-rows-[1fr,auto] h-full">
				<ChatLog selectedChannel={selectedChannel} />
				<div className="px-2 py-2 min-h-16">
					<textarea
						placeholder="Type a message here..."
						className="px-1 h-16 w-full input rounded bg-darkSidebar text-darkText"
					/>
				</div>
			</div>
		</div>
	);
}
