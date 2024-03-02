"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import { redirect } from "next/navigation";
import { createContext, useState } from "react";

export const themeContext = createContext({
	lightMode: false,
	setLightMode: (value: boolean) => {},
});

var hasAccount = false;

export default function Home() {
	if (!hasAccount) {
		redirect("/signUp");
	}
	const [lightMode, setLightMode] = useState(false);

	return (
		<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
			<div className="">
				<ChatSidebar />
				<div className="col-span-2 text-white font-bold ">
					<ChatLog />
					<div style={{ padding: "10px 0", position: "relative", left: "1%", width: "95%" }}>
						<textarea placeholder="Type a message here..." color="neutral" />
					</div>
				</div>
			</div>

			<div className="col-span-2 text-white font-bold ">
				<ChatLog />
			</div>
		</div>
	);
}
