"use client";

import ChatLog from "@/components/chatLog";
import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import { useRspc } from "@/integration";
import { redirect } from "next/navigation";
import { createContext, useContext, useState } from "react";

import { LoginContext } from "./loginContext";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

export default function Home() {
	const [selectedChannel, setSelectedChannel] = useState(-1);
	const { isLoggedIn } = useContext(LoginContext);
	const [inputValue, setInputValue] = useState("");

	if (!isLoggedIn) {
		console.log("in main page");
		redirect("/signIn");
	}

	const [lightMode, setLightMode] = useState(false);
	const rspc = useRspc();
	const sendMsg = rspc.useMutation("channel.send");

	const keyDown = (event) => {
		console.log("pressed something");
		if (inputValue.length > 0 && event.key == "Enter") {
			sendMsg.mutate({ channel_id: selectedChannel, content: inputValue });
			console.log("pressed enter");
		}
	};

	return (
		<div className="grid grid-cols-[auto,1fr] h-[calc(100vh-60px)]">
			<ChatSidebar selectedChannel={selectedChannel} setSelectedChannel={setSelectedChannel} />
			<div className="grid grid-rows-[1fr,auto] h-full">
				<ChatLog selectedChannel={selectedChannel} />
				<div className="px-2 py-2 min-h-16">
					<textarea
						value={inputValue}
						onKeyDown={keyDown}
						placeholder="Type a message here..."
						className="px-1 h-16 w-full input rounded bg-darkSidebar text-darkText"
						onChange={(e) => setInputValue(e.target.value)}
					/>
				</div>
			</div>
		</div>
	);
}
