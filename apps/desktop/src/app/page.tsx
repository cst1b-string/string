"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import { rspc } from "@/integration";

export function Home() {
	rspc.useQuery(["settings.theme"]);

	return (
		<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
			<div className="">
				<ChatSidebar />
			</div>

			<div className="col-span-2 text-white font-bold ">
				<ChatLog />
			</div>
		</div>
	);
}
