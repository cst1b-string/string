"use client";

import ChatSidebar from "@/components/chatSidebar";
import ChatLog from "@/components/chatlog";
import Textarea from "@mui/joy/Textarea";

export default function Home() {
	return (
		<div className="grid grid-cols-3 divide-x divide-gray-400 h-[calc(100vh-80px)]">
			<div className="">
				<ChatSidebar />
				<div className="col-span-2 text-white font-bold ">
					<ChatLog />
					{/* <div style={{ padding: "10px 0", position: "relative", left: "1%", width: "95%" }}>
						<textarea placeholder="Type a message here..." color="neutral" />
					</div> */}
				</div>
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
	);
}
