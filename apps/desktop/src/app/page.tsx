"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import { redirect } from "next/navigation";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

var hasAccount = true;

export default function Home() {
	const [selectedChannel, setSelectedChannel] = useState(-1);

	if (!hasAccount) {
		redirect("/signUp");
	}
	const [lightMode, setLightMode] = useState(false);

	return (
		<div className="grid grid-cols-[250px,1fr] divide-x divide-gray-400 h-[calc(100vh-80px)]">
			<div className="">
				<ChatSidebar selectedChannel={selectedChannel} setSelectedChannel={setSelectedChannel} />
			</div>
			<div className="">
				<ChatLog />
				<div style={{ padding: "10px 0", position: "relative", left: "1%", width: "95%" }}>
					<textarea placeholder="Type a message here..." color="neutral" />
				</div>
			</div>
		</div>
	);
}
